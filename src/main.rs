#![deny(clippy::all)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use std::{
    array, env, fs,
    io::{Read, Write},
    mem,
    net::UdpSocket,
    path::{Path, PathBuf},
};

use eframe::egui::{self, Ui};

use crossbeam::channel;
use enum_iterator::all;
use gcfeeder::{
    adapter::{poller::Poller, Port},
    config::{Config, Profile},
    feeder::Feeder,
};
use log::warn;
use simple_logger::SimpleLogger;
use trayicon::{MenuBuilder, TrayIconBuilder};

pub fn main() {
    std::panic::set_hook(Box::new(panic::hook));

    let exe_path = env::current_exe().expect("Failed to get current exe path");
    env::set_current_dir(
        exe_path
            .parent()
            .expect("Failed to get current exe parent path"),
    )
    .expect("Failed to set current working directory");

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

type Usb = rusb::GlobalContext;

struct MyApp {
    config: Config,
    config_path: PathBuf,
    ctrlc_reciever: channel::Receiver<()>,
    tray_reciever: channel::Receiver<TrayMessage>,
    poller: Poller<Usb>,
    feeders: [(Feeder<Usb>, Option<UdpSocket>); Port::COUNT],
}

impl MyApp {
    const CONFIG_PATH: &'static str = "gcfeeder.toml";

    pub fn new(
        ctrlc_reciever: channel::Receiver<()>,
        tray_reciever: channel::Receiver<TrayMessage>,
    ) -> Self {
        let config_path = Path::new(Self::CONFIG_PATH).to_path_buf();

        let config = Self::load_or_create_config(&config_path);
        let poller = Poller::new(Usb {});
        let feeders = Self::feeders_from_config(&config, &poller);

        Self {
            config,
            config_path,
            ctrlc_reciever,
            tray_reciever,
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

    fn feeders_from_config(
        config: &Config,
        poller: &Poller<Usb>,
    ) -> [(Feeder<Usb>, Option<UdpSocket>); Port::COUNT] {
        array::from_fn(|i| Self::feeder_from_config(config, poller, i.try_into().unwrap()))
    }

    fn feeder_from_config(
        config: &Config,
        poller: &Poller<Usb>,
        port: Port,
    ) -> (Feeder<Usb>, Option<UdpSocket>) {
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

        (feeder, socket)
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

        ui.label(format!("Average poll time: {}ms", poll_avg));

        for port in all::<Port>() {
            let (feeder, _) = &self.feeders[port.index()];

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
                if let Some(_config) = Self::load_config(&self.config_path) {
                    // TODO: Send config update to feeder instead of re-creating it. Be able
                    // to handle re-creating on feed callback when making a new feeder.
                    todo!();
                }
            }

            if ui.button("Save").clicked() {
                Self::write_config(&self.config, &self.config_path);
            }
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if panic::panicked() {
            frame.close();
        }

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

mod panic {
    use std::{
        fs, io,
        panic::PanicInfo,
        sync::atomic::{AtomicBool, Ordering},
        thread,
    };

    use backtrace::Backtrace;

    static PANICKED: AtomicBool = AtomicBool::new(false);

    /// Attempts to log panic information with a backtrace to `panic.log` in
    /// the current working directory.
    ///
    /// Implementation adapted from Rust's default panic hook.
    pub fn hook(info: &PanicInfo<'_>) {
        if let Ok(mut log) = fs::File::create("panic.log") {
            // The current implementation always returns `Some`.
            let location = info.location().unwrap();

            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<dyn Any>",
                },
            };
            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");

            let write = |err: &mut dyn io::Write| {
                let _ = writeln!(err, "thread '{name}' panicked at '{msg}', {location}");

                let backtrace = Backtrace::new();
                let _ = writeln!(err, "stack backtrace:");
                let _ = err.write_all(format!("{:#?}", backtrace).as_bytes());
            };

            write(&mut log);
        }

        PANICKED.store(true, Ordering::Release);
    }

    pub fn panicked() -> bool {
        PANICKED.load(Ordering::Acquire)
    }
}
