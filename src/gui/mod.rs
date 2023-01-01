use std::env;

use app::{App, TrayMessage};
use crossbeam::channel;
use egui::Color32;
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
        .build()
        .unwrap()
        .init()
        .expect("Failed to set logger");

    let options = eframe::NativeOptions {
        initial_window_size: Some([600.0, 420.0].into()),

        ..Default::default()
    };

    const ICON: &[u8] = include_bytes!("../../resource/icon.ico");
    let (tray_tx, tray_rx) = channel::unbounded();

    let tray_icon = TrayIconBuilder::new()
        .sender_crossbeam(tray_tx)
        .icon_from_buffer(ICON)
        .tooltip("gcfeeder")
        .menu(
            MenuBuilder::new()
                .item("Show", TrayMessage::Show)
                .item("Hide", TrayMessage::Hide)
                .item("Exit", TrayMessage::Exit),
        )
        .build()
        .unwrap();

    eframe::run_native(
        format!("gcfeeder | {}", env!("VERSION")).as_str(),
        options,
        Box::new(move |_cc| Box::new(App::new(tray_icon, tray_rx, log_rx))),
    );
}
