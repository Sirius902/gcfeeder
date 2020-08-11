#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::{window, Application, Settings};
use ui::GCFeeder;

mod ui;

fn main() {
    GCFeeder::run(Settings {
        window: window::Settings {
            size: (640, 480),
            resizable: false,
            decorations: true,
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
