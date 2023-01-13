#![deny(clippy::all)]
#![windows_subsystem = "windows"]

use std::env;

use gcfeeder::{gui, panic};

pub fn main() {
    std::panic::set_hook(Box::new(panic::hook));

    let exe_path = env::current_exe().expect("Failed to get current exe path");
    env::set_current_dir(
        exe_path
            .parent()
            .expect("Failed to get current exe parent path"),
    )
    .expect("Failed to set current working directory");

    gui::run();
}
