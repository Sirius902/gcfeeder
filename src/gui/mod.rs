use std::{env, mem};

use app::{App, TrayMessage};
use crossbeam::channel;
use trayicon::{MenuBuilder, TrayIconBuilder};

mod app;
pub mod log;
mod util;

pub fn run() {
    let (tx, rx) = channel::bounded(1);

    mem::drop(ctrlc::set_handler(move || {
        let _ = tx.try_send(());
    }));

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
                .item("Minimize", TrayMessage::Minimize)
                .item("Restore", TrayMessage::Restore)
                .item("Exit", TrayMessage::Exit),
        )
        .build()
        .unwrap();

    eframe::run_native(
        format!("gcfeeder | {}", env!("VERSION")).as_str(),
        options,
        Box::new(move |_cc| Box::new(App::new(rx, tray_icon, tray_rx, log_rx))),
    );
}
