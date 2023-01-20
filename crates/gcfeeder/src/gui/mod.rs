use std::env;

use app::App;
#[cfg(windows)]
use app::TrayMessage;
use crossbeam::channel;
use egui::Color32;
use image::EncodableLayout;

#[cfg(windows)]
use trayicon::{MenuBuilder, TrayIconBuilder};

mod app;
pub mod log;
mod util;

const ERROR_COLOR: Color32 = Color32::from_rgb(197, 15, 31);
const WARN_COLOR: Color32 = Color32::from_rgb(193, 156, 0);
const INFO_COLOR: Color32 = Color32::from_rgb(58, 150, 221);
const DEBUG_COLOR: Color32 = Color32::from_rgb(136, 23, 152);

pub fn run() {
    let (log_tx, log_rx) = channel::unbounded();
    log::LoggerBuilder::new()
        .sender(log_tx)
        .with_level(::log::LevelFilter::Trace)
        .build()
        .unwrap()
        .init()
        .expect("Failed to set logger");

    #[cfg(target_os = "linux")]
    ::log::warn!("Hiding the window is not supported on Wayland");

    const ICON_FILE: &[u8] = include_bytes!("../../resource/icon.png");

    let icon = image::load_from_memory(ICON_FILE).unwrap();
    let icon_data = icon.into_rgba8();
    let icon_dim = icon_data.dimensions();

    let options = eframe::NativeOptions {
        initial_window_size: Some([600.0, 420.0].into()),
        icon_data: Some(eframe::IconData {
            rgba: icon_data.as_bytes().to_vec(),
            width: icon_dim.0,
            height: icon_dim.1,
        }),

        ..Default::default()
    };

    #[cfg(windows)]
    let (_tray_icon, tray_rx) = {
        const ICON_ICO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.ico"));

        let (tray_tx, tray_rx) = channel::unbounded();

        let tray_icon = TrayIconBuilder::new()
            .sender_crossbeam(tray_tx)
            .icon_from_buffer(ICON_ICO)
            .tooltip("gcfeeder")
            .menu(
                MenuBuilder::new()
                    .item("Show", TrayMessage::Show)
                    .item("Hide", TrayMessage::Hide)
                    .item("Exit", TrayMessage::Exit),
            )
            .build()
            .unwrap();

        (tray_icon, tray_rx)
    };

    let version_string = if !env!("VERSION").is_empty() {
        env!("VERSION")
    } else {
        concat!("g", env!("VERGEN_GIT_SHA_SHORT"))
    };

    eframe::run_native(
        format!("gcfeeder | {}", version_string).as_str(),
        options,
        #[cfg(windows)]
        Box::new(move |_cc| Box::new(App::new(tray_rx, log_rx))),
        #[cfg(not(windows))]
        Box::new(move |_cc| Box::new(App::new(log_rx))),
    );
}
