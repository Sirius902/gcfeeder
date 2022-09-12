#![deny(clippy::all)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use std::env;

use gcfeeder::{gui, panic};
use simple_logger::SimpleLogger;

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

    gui::run();
}
