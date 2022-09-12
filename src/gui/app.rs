use std::{
    array, fs,
    io::{Read, Write},
    net::UdpSocket,
    path::{Path, PathBuf},
};

use eframe::egui::{self, Ui};
use trayicon::TrayIcon;

use super::log::Message as LogMessage;
use crate::{
    adapter::{poller::Poller, Port},
    config::{Config, Profile},
    feeder::Feeder,
    panic,
};
use crossbeam::channel;
use enum_iterator::all;
use log::{info, warn};

type Usb = rusb::GlobalContext;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TrayMessage {
    Minimize,
    Restore,
    Exit,
}

pub struct App {
    config: Config,
    config_path: PathBuf,
    ctrlc_receiver: channel::Receiver<()>,
    _tray_icon: TrayIcon<TrayMessage>,
    tray_receiver: channel::Receiver<TrayMessage>,
    log_receiver: channel::Receiver<LogMessage>,
    log_messages: Vec<LogMessage>,
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
            config,
            config_path,
            ctrlc_receiver,
            _tray_icon: tray_icon,
            tray_receiver,
            log_receiver,
            log_messages: Vec::new(),
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

    fn log_ui(&mut self, ui: &mut Ui) {
        ui.set_min_height(100.0);
        ui.heading("Log");

        while let Ok(message) = self.log_receiver.try_recv() {
            self.log_messages.push(message);
        }

        for message in self.log_messages.iter() {
            message.draw(ui);
        }
    }

    fn calibration_ui(&mut self, ui: &mut Ui) {
        ui.set_min_width(200.0);
        ui.heading("Calibration");

        let poll_avg = self
            .poller
            .average_poll_time()
            .filter(|_| self.poller.connected())
            .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "-".to_owned());

        ui.label(format!("Average poll time: {}ms", poll_avg));

        ui.spacing();

        for port in all::<Port>() {
            let feeder = &self.feeders[port.index()];

            let feed_avg = feeder
                .average_feed_time()
                .filter(|_| feeder.connected())
                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                .unwrap_or_else(|| "-".to_owned());

            ui.label(format!("Port {:?}", port));
            ui.label(format!("Average feed time: {}ms", feed_avg));
        }
    }

    fn config_ui(&mut self, ui: &mut Ui) {
        ui.heading("Config");

        ui.horizontal(|ui| {
            if ui.button("Reload").clicked() {
                if let Some(config) = Self::load_config(&self.config_path) {
                    // TODO: Send config update to feeder instead of re-creating it.
                    self.feeders = Self::feeders_from_config(&config, &self.poller);
                    self.config = config;
                    info!("Reloaded config");
                }
            }

            if ui.button("Save").clicked() {
                Self::write_config(&self.config, &self.config_path);
                info!("Saved config");
            }
        });
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
                    if ui.button("Exit").clicked() {
                        frame.close();
                    }

                    if ui.button("Reset Layout").clicked() {
                        *ui.ctx().memory() = Default::default();
                        ui.close_menu();
                    }
                });

                if ui.button("Minimize").clicked() {
                    frame.set_visible(false);
                }
            });
        });

        egui::TopBottomPanel::bottom("log_panel").show(ctx, |ui| {
            self.log_ui(ui);
        });

        egui::SidePanel::left("calibration_panel").show(ctx, |ui| {
            self.calibration_ui(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.config_ui(ui);
        });
    }
}
