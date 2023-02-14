#![deny(clippy::all)]
#![windows_subsystem = "windows"]

use std::{env, error::Error};

use gcfeeder::gui;

pub fn main() -> Result<(), Box<dyn Error>> {
    std::panic::set_hook(Box::new(panic_log::hook));

    let exe_path = env::current_exe().expect("Failed to get current exe path");
    env::set_current_dir(
        exe_path
            .parent()
            .expect("Failed to get current exe parent path"),
    )
    .expect("Failed to set current working directory");

    gui::run()?;
    Ok(())
}
