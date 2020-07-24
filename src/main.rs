use druid::{AppLauncher, PlatformError, WindowDesc};

mod ui;

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui::builder).title("gcfeeder");

    let state = ui::AppState::new();
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)
}
