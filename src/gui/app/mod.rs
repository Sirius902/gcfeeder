use std::{
    array, fs,
    io::{Read, Write},
    net::UdpSocket,
    path::{Path, PathBuf},
};

use eframe::egui;
use trayicon::TrayIcon;

use self::panel::StatsPanel;

use super::log::Message as LogMessage;
use crate::{
    adapter::{poller::Poller, Port},
    config::{Config, Profile},
    feeder::Feeder,
    panic,
};
use crossbeam::channel;
use log::{info, warn};
use panel::{CalibrationPanel, ConfigEditor, LogPanel};

mod panel;

type Usb = rusb::GlobalContext;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TrayMessage {
    Minimize,
    Restore,
    Exit,
}

pub struct App {
    log_panel: LogPanel,
    stats_open: bool,
    config: Config,
    config_path: PathBuf,
    ctrlc_receiver: channel::Receiver<()>,
    _tray_icon: TrayIcon<TrayMessage>,
    tray_receiver: channel::Receiver<TrayMessage>,
    poller: Poller<Usb>,
    feeders: [Feeder<Usb>; Port::COUNT],
}

impl App {
    const CONFIG_PATH: &'static str = "gcfeeder.toml";

    pub fn new(
        ctrlc_receiver: channel::Receiver<()>,
        tray_icon: TrayIcon<TrayMessage>,
        tray_receiver: channel::Receiver<TrayMessage>,
        log_receiver: channel::Receiver<LogMessage>,
    ) -> Self {
        let config_path = Path::new(Self::CONFIG_PATH).to_path_buf();

        let config = Self::load_or_create_config(&config_path);
        let poller = Poller::new(Usb {});
        let feeders = Self::feeders_from_config(&config, &poller);

        Self {
            log_panel: LogPanel::new(log_receiver),
            stats_open: false,
            config,
            config_path,
            ctrlc_receiver,
            _tray_icon: tray_icon,
            tray_receiver,
            poller,
            feeders,
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
        let mut f = match fs::File::open(config_path) {
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
        let mut f = match fs::File::create(config_path) {
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

    fn feeders_from_config(config: &Config, poller: &Poller<Usb>) -> [Feeder<Usb>; Port::COUNT] {
        array::from_fn(|i| Self::feeder_from_config(config, poller, i.try_into().unwrap()))
    }

    fn feeder_from_config(config: &Config, poller: &Poller<Usb>, port: Port) -> Feeder<Usb> {
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

        let feeder = Feeder::new(profile, poller.add_listener(port));

        let socket = {
            let server_config = &config.input_server[index];
            if server_config.enabled {
                UdpSocket::bind("0.0.0.0:0")
                    .and_then(|s| s.connect(("127.0.0.1", server_config.port)).map(|()| s))
                    .map(Option::Some)
                    .unwrap_or_else(|e| {
                        warn!(
                            "Failed to connect to input server on localhost:{} set for port {:?}: {}",
                            server_config.port, port, e
                        );
                        None
                    })
            } else {
                None
            }
        };

        if let Some(socket) = socket {
            feeder.on_feed(move |record| {
                if let Some(input) = record.raw_input.as_ref() {
                    let bytes =
                        bincode::serialize(input).expect("Failed to serialize Input to bytes");
                    let _ = socket.send(&bytes);
                }
            });
        }

        feeder
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if panic::panicked() {
            frame.close();
        }

        if let Ok(message) = self.tray_receiver.try_recv() {
            match message {
                TrayMessage::Minimize => frame.set_visible(false),
                TrayMessage::Restore => frame.set_visible(true),
                TrayMessage::Exit => frame.close(),
            }
        }

        if self.ctrlc_receiver.try_recv() == Ok(()) {
            frame.close();
        }

        ctx.request_repaint();

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

                if ui.button("Minimize").clicked() {
                    frame.set_visible(false);
                }
            });
        });

        egui::Window::new("Stats")
            .open(&mut self.stats_open)
            .show(ctx, |ui| {
                StatsPanel::new(&mut self.poller, &mut self.feeders).ui(ui);
            });

        egui::TopBottomPanel::bottom("log_panel").show(ctx, |ui| {
            self.log_panel.ui(ui);
        });

        egui::SidePanel::left("calibration_panel").show(ctx, |ui| {
            CalibrationPanel::new(&mut self.feeders).ui(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let state = {
                let mut config_editor = ConfigEditor::new(&mut self.config);
                config_editor.ui(ui);
                config_editor.into_state()
            };

            if state.reload() {
                if let Some(config) = Self::load_config(&self.config_path) {
                    // TODO: Send config update to feeder instead of re-creating it.
                    self.feeders = Self::feeders_from_config(&config, &self.poller);
                    self.config = config;
                    info!("Reloaded config");
                }
            }

            if state.save() {
                Self::write_config(&self.config, &self.config_path);
                info!("Saved config");
            }
        });
    }
}
