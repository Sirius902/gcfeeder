#![deny(clippy::all)]
// TODO: Uncomment after https://github.com/rust-windowing/glutin/issues/1467 is resolved.
// #![cfg_attr(
//     all(target_os = "windows", not(debug_assertions)),
//     windows_subsystem = "windows"
// )]

use std::{
    env, fs,
    io::{Read, Write},
    mem,
    path::{Path, PathBuf},
};

use eframe::egui::{self, Ui};

use crossbeam::channel;
use gcfeeder::{
    adapter::{poller::Poller, Port},
    config::{Config, Profile},
    feeder::Feeder,
};
use log::warn;
use simple_logger::SimpleLogger;
use trayicon::{MenuBuilder, TrayIconBuilder};

// TODO: Add a panic handler?, handle mutex poisoning.
pub fn main() {
    SimpleLogger::new()
        .with_local_timestamps()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .expect("failed to set logger");

    let options = eframe::NativeOptions {
        initial_window_size: Some([600.0, 420.0].into()),

        ..Default::default()
    };

    let (tx, rx) = channel::bounded(1);

    mem::drop(ctrlc::set_handler(move || {
        let _ = tx.try_send(());
    }));

    const ICON: &[u8] = include_bytes!("../resource/icon.ico");
    let (tray_tx, tray_rx) = channel::unbounded();

    let _tray_icon = TrayIconBuilder::new()
        .sender_crossbeam(tray_tx)
        .icon_from_buffer(ICON)
        .tooltip("gcfeeder")
        .menu(
            MenuBuilder::new()
                .item("Minimize", TrayMessage::Minimize)
                .item("Restore", TrayMessage::Restore)
                .item("Exit", TrayMessage::Exit),
        )
        .build()
        .unwrap();

    eframe::run_native(
        format!("gcfeeder | {}", env!("VERSION")).as_str(),
        options,
        Box::new(move |_cc| Box::new(MyApp::new(rx, tray_rx))),
    );
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum TrayMessage {
    Minimize,
    Restore,
    Exit,
}

struct MyApp {
    config: Config,
    config_path: PathBuf,
    ctrlc_reciever: channel::Receiver<()>,
    tray_reciever: channel::Receiver<TrayMessage>,
    poller: Poller<rusb::GlobalContext>,
    feeder: Feeder<rusb::GlobalContext>,
}

impl MyApp {
    const CONFIG_PATH: &'static str = "gcfeeder.toml";

    pub fn new(
        ctrlc_reciever: channel::Receiver<()>,
        tray_reciever: channel::Receiver<TrayMessage>,
    ) -> Self {
        let exe_path = env::current_exe().expect("Failed to get current exe path");
        let mut config_path =
            PathBuf::from(exe_path.parent().expect("Failed to get exe path parent"));
        config_path.push(Self::CONFIG_PATH);

        let config = Self::load_or_create_config(&config_path);
        let profile = {
            let selected = &config.profile.selected[Port::One.index()];
            config
                .profile
                .list
                .get(selected)
                .cloned()
                .unwrap_or_else(|| {
                    warn!("Missing profile \'{}\', using default", selected);
                    Profile::default()
                })
        };

        let poller = Poller::new(rusb::GlobalContext {});
        let feeder = Feeder::new(profile, poller.add_listener(Port::One));

        Self {
            config,
            config_path,
            ctrlc_reciever,
            tray_reciever,
            poller,
            feeder,
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

    fn log_ui(&mut self, ui: &mut Ui) {
        ui.heading("Log");
    }

    fn calibration_ui(&mut self, ui: &mut Ui) {
        ui.heading("Calibration");

        let poll_avg = self
            .poller
            .average_poll_time()
            .filter(|_| self.poller.connected())
            .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "-".to_owned());

        let feed_avg = self
            .feeder
            .average_feed_time()
            .filter(|_| self.feeder.connected())
            .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "-".to_owned());

        ui.label(format!("Average poll time: {}ms", poll_avg));
        ui.label(format!("Average feed time: {}ms", feed_avg));
    }

    fn config_ui(&mut self, ui: &mut Ui) {
        ui.heading("Config");

        if ui.button("Reload Config").clicked() {
            if let Some(_config) = Self::load_config(&self.config_path) {
                // TODO: Send config update to feeder instead of re-creating it. Be able
                // to handle re-creating on feed callback when making a new feeder.
                todo!();
            }
        }

        if ui.button("Save Config").clicked() {
            Self::write_config(&self.config, &self.config_path);
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(message) = self.tray_reciever.try_recv() {
            match message {
                TrayMessage::Minimize => frame.set_visible(false),
                TrayMessage::Restore => frame.set_visible(true),
                TrayMessage::Exit => frame.close(),
            }
        }

        if self.ctrlc_reciever.try_recv() == Ok(()) {
            frame.close();
        }

        ctx.request_repaint();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Exit").clicked() {
                        frame.close();
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
