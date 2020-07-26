use iced::{window, Sandbox, Settings};
use ui::GCFeeder;

mod ui;

fn main() {
    GCFeeder::run(Settings {
        window: window::Settings {
            size: (640, 480),
            resizable: false,
            decorations: true,
        },
        ..Settings::default()
    })
}
