use druid::{AppLauncher, PlatformError, WindowDesc};
use gcfeeder::ui;

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui::builder).title("gcfeeder");

    let state = ui::State::new();
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)
}
