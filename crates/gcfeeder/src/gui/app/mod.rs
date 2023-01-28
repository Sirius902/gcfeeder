use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    net::UdpSocket,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use eframe::egui;
use enum_iterator::all;

use self::panel::calibration::ConfigUpdate;

use super::log::Message as LogMessage;
use crate::config::{Config, Profile};
use crossbeam::channel;
use gcfeeder_core::{
    adapter::{source::InputSource, Port},
    feeder::{self, Feeder, Record},
    util::recent_channel::{self as recent, TryRecvError},
};
use log::{info, warn};
use panel::{
    calibration::State as CalibrationState, config::Message as ConfigMessage,
    config::State as ConfigState, profile::Message as ProfileMessage,
    profile::State as ProfileState, CalibrationPanel, ConfigEditor, LogPanel, ProfilePanel,
    StatsPanel,
};

mod panel;
mod widget;

#[cfg(windows)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TrayMessage {
    Show,
    Hide,
    Exit,
}

pub struct App<S: InputSource + 'static> {
    log_panel: LogPanel,
    calibration_state: Option<CalibrationState>,
    config_state: Option<ConfigState>,
    profile_state: Option<ProfileState>,
    editor_profile: Option<String>,
    stats_open: bool,
    config: Config,
    config_path: PathBuf,
    #[cfg(windows)]
    tray_receiver: channel::Receiver<TrayMessage>,
    hidden: bool,
    input_source: S,
    feeders: [Feeder<S::Listener>; Port::COUNT],
    receivers: [feeder::Receiver; Port::COUNT],
    records: [Option<Record>; Port::COUNT],
}

impl<S: InputSource + 'static> App<S> {
    pub fn new(
        input_source: S,
        #[cfg(windows)] tray_receiver: channel::Receiver<TrayMessage>,
        log_receiver: channel::Receiver<LogMessage>,
    ) -> Self {
        let config_path = dirs::config_dir()
            .expect("Failed to get config directory")
            .join("gcfeeder")
            .join("gcfeeder.toml");

        let config = Self::load_or_create_config(&config_path);
        let (feeders, receivers) = Self::feeders_from_config(&config, &input_source);

        Self {
            log_panel: LogPanel::new(log_receiver),
            calibration_state: None,
            config_state: None,
            profile_state: None,
            editor_profile: None,
            stats_open: false,
            config,
            config_path,
            #[cfg(windows)]
            tray_receiver,
            hidden: false,
            input_source,
            feeders,
            receivers,
            records: Default::default(),
        }
    }

    fn load_or_create_config(config_path: impl AsRef<Path>) -> Config {
        let config_path = config_path.as_ref();

        if config_path.exists() {
            Self::load_config(config_path).unwrap_or_default()
        } else {
            let config = Config::default();
            Self::write_config(&config, config_path);
            config
        }
    }

    fn load_config(config_path: impl AsRef<Path>) -> Option<Config> {
        let mut f = match fs::File::open(&config_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open config file: {}", e);
                return None;
            }
        };

        let mut s = String::new();
        match f.read_to_string(&mut s) {
            Ok(_) => (),
            Err(e) => {
                warn!("Failed to read config file: {}", e);
                return None;
            }
        }

        let config = toml::from_str::<Config>(s.as_str());

        if let Err(e) = config.as_ref() {
            warn!("Failed to parse config file: {}", e);
        }

        config.ok()
    }

    fn write_config(config: &Config, config_path: impl AsRef<Path>) {
        if let Err(e) = fs::create_dir_all(
            config_path
                .as_ref()
                .parent()
                .expect("Config path has no parent path"),
        ) {
            warn!("Failed to create config directory: {}", e);
            return;
        }

        let mut f = match fs::File::create(&config_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to create config file: {}", e);
                return;
            }
        };

        let s = match toml::to_string(&config) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to serialize config: {}", e);
                return;
            }
        };

        match f.write_all(s.as_bytes()) {
            Ok(()) => (),
            Err(e) => {
                warn!("Failed to write config file: {}", e);
            }
        }
    }

    fn feeders_from_config(
        config: &Config,
        input_source: &S,
    ) -> (
        [Feeder<S::Listener>; Port::COUNT],
        [feeder::Receiver; Port::COUNT],
    ) {
        let mut feeders: [Option<Feeder<S::Listener>>; Port::COUNT] = Default::default();
        let mut receivers: [Option<feeder::Receiver>; Port::COUNT] = Default::default();

        for port in all::<Port>() {
            let index = port.index();
            let (feeder, receiver) = Self::feeder_from_config(config, input_source, port);
            feeders[index] = Some(feeder);
            receivers[index] = Some(receiver);
        }

        (feeders.map(Option::unwrap), receivers.map(Option::unwrap))
    }

    fn feeder_from_config(
        config: &Config,
        input_source: &S,
        port: Port,
    ) -> (Feeder<S::Listener>, feeder::Receiver) {
        let index = port.index();
        let profile = {
            let selected = &config.profile.selected[index];
            config
                .profile
                .list
                .get(selected)
                .cloned()
                .unwrap_or_else(|| {
                    warn!(
                        "Missing profile \'{}\' set for port {:?}, using default",
                        selected, port
                    );
                    Profile::default()
                })
        };

        let feeder = Feeder::new(profile, input_source.add_listener(port));

        let socket = {
            let server_config = &config.input_server[index];
            if server_config.enabled {
                UdpSocket::bind(("127.0.0.1", server_config.port))
                    .and_then(|s| s.set_nonblocking(true).map(|()| s))
                    .map(Option::Some)
                    .unwrap_or_else(|e| {
                        warn!(
                            "Failed to create input server on localhost:{} set for port {:?}: {}",
                            server_config.port, port, e
                        );
                        None
                    })
            } else {
                None
            }
        };

        if let Some(socket) = socket {
            let mut clients = HashMap::new();

            feeder.on_feed(move |record| {
                loop {
                    match socket.recv_from(&mut []) {
                        Ok((received, src_addr)) => {
                            if received == 0 {
                                clients.insert(src_addr, Instant::now());
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(_) => {}
                    }
                }

                if let Some(input) = record.raw_input.as_ref() {
                    let bytes =
                        bincode::serialize(input).expect("Failed to serialize Input to bytes");
                    let _ = socket.send(&bytes);

                    clients.retain(|&sock_addr, last_msg| {
                        if last_msg.elapsed() > Duration::from_secs(60) {
                            return false;
                        }

                        match socket.send_to(&bytes, sock_addr) {
                            Ok(_) => true,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => true,
                            Err(_) => false,
                        }
                    });
                }
            });
        }

        let (tx, rx) = recent::channel();
        feeder.send_on_feed(tx);
        (feeder, rx)
    }

    fn handle_messages(&mut self, frame: &mut eframe::Frame) {
        if panic_log::panicked() {
            frame.close();
        }

        #[cfg(windows)]
        while let Ok(message) = self.tray_receiver.try_recv() {
            match message {
                TrayMessage::Show => {
                    frame.set_visible(true);
                    self.hidden = false;
                }
                TrayMessage::Hide => {
                    frame.set_visible(false);
                    self.hidden = true;
                }
                TrayMessage::Exit => frame.close(),
            }
        }

        for (i, (feeder, receiver)) in self
            .feeders
            .iter()
            .zip(self.receivers.iter_mut())
            .enumerate()
        {
            if !feeder.connected() {
                self.records[i] = None;
                continue;
            }

            match receiver.try_recv() {
                Ok(record) => {
                    self.records[i] = Some(record);
                }
                Err(TryRecvError::Disconnected) => {
                    warn!("Feeder receiver disconnected while in use");
                    let (tx, rx) = recent::channel();
                    feeder.send_on_feed(tx);
                    *receiver = rx;
                }
                _ => (),
            }
        }
    }

    pub fn save_config(&mut self) {
        Self::write_config(&self.config, &self.config_path);
        info!("Saved config");

        if let Some(state) = self.config_state.as_mut() {
            state.notify_clean();
        }
    }

    pub fn reload_config(&mut self) {
        if let Some(config) = Self::load_config(&self.config_path) {
            // TODO: Send config update to feeder instead of re-creating it.
            for feeder in &self.feeders {
                feeder.clear_callbacks();
            }

            let (feeders, receivers) = Self::feeders_from_config(&config, &self.input_source);
            self.feeders = feeders;
            self.receivers = receivers;
            self.config = config;
            info!("Reloaded config");

            if let Some(state) = self.config_state.as_mut() {
                state.notify_clean();
            }
        }
    }
}

impl<S: InputSource> eframe::App for App<S> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_messages(frame);

        if !self.hidden {
            ctx.request_repaint();
        }

        if let Some(editor_profile) = self.editor_profile.as_mut() {
            let mut message: Option<ProfileMessage> = None;

            egui::CentralPanel::default().show(ctx, |ui| {
                let mut panel = ProfilePanel::new(
                    &mut self.config,
                    editor_profile.as_str(),
                    self.profile_state.take(),
                );
                panel.ui(ui);

                let (state, m) = panel.into_state();
                message = m;
                self.profile_state = Some(state);
            });

            match message {
                Some(ProfileMessage::SaveReload) => {
                    self.profile_state = None;
                    self.editor_profile = None;
                    self.save_config();
                    self.reload_config();
                }
                Some(ProfileMessage::Cancel) => {
                    self.profile_state = None;
                    self.editor_profile = None;
                }
                None => {}
            }

            return;
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Reset Layout").clicked() {
                        *ui.ctx().memory() = Default::default();
                        ui.close_menu();
                    }

                    if ui.button("Exit").clicked() {
                        frame.close();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.toggle_value(&mut self.stats_open, "Stats").clicked() {
                        ui.close_menu();
                    }
                });

                if ui.button("Hide").clicked() {
                    frame.set_visible(false);
                    self.hidden = true;
                }
            });
        });

        egui::Window::new("Stats")
            .open(&mut self.stats_open)
            .show(ctx, |ui| {
                StatsPanel::new(&mut self.input_source, &mut self.feeders).ui(ui);
            });

        egui::TopBottomPanel::bottom("log_panel").show(ctx, |ui| {
            self.log_panel.ui(ui);
        });

        egui::SidePanel::left("calibration_panel").show(ctx, |ui| {
            let mut panel = CalibrationPanel::new(
                &mut self.feeders,
                &self.records,
                &self.config,
                self.calibration_state.take(),
            );
            panel.ui(ui);
            let (state, update) = panel.into_state();
            self.calibration_state = Some(state);

            if let Some(update) = update {
                let profile = self
                    .config
                    .profile
                    .selected_mut(update.port())
                    .expect("active profile should exist");

                match update {
                    ConfigUpdate::SticksCalibration { calibration: s, .. } => {
                        profile.calibration.stick_data = Some(s);
                        profile.calibration.enabled = true;
                    }
                    ConfigUpdate::TriggersCalibration { calibration: t, .. } => {
                        profile.calibration.trigger_data = Some(t);
                        profile.calibration.enabled = true;
                    }
                }

                self.save_config();
                self.reload_config();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let prev_state = self.config_state.take();
            let state = self.config_state.insert({
                let mut config_editor = ConfigEditor::new(&mut self.config, prev_state);
                config_editor.ui(ui);
                config_editor.into_state()
            });

            match state.message() {
                Some(ConfigMessage::Reload) => self.reload_config(),
                Some(ConfigMessage::Save) => {
                    self.save_config();
                    self.reload_config();
                }
                Some(ConfigMessage::EditProfile { name }) => {
                    self.editor_profile = Some(name);
                }
                None => {}
            }
        });
    }
}
