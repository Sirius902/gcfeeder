#![deny(clippy::all)]
use std::{
    fs, io,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

use backtrace::Backtrace;

static PANICKED: AtomicBool = AtomicBool::new(false);

/// Attempts to log panic information with a backtrace to `panic.log` in
/// the current working directory.
///
/// Implementation adapted from Rust's default panic hook.
pub fn hook(info: &PanicInfo<'_>) {
    if let Ok(mut log) = fs::File::create("panic.log") {
        // The current implementation always returns `Some`.
        let location = info.location().unwrap();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };
        let thread = thread::current();
        let name = thread.name().unwrap_or("<unnamed>");

        let write = |err: &mut dyn io::Write| {
            let _ = writeln!(err, "thread '{name}' panicked at '{msg}', {location}");

            let backtrace = Backtrace::new();
            let _ = writeln!(err, "stack backtrace:");
            let _ = err.write_all(format!("{:#?}", backtrace).as_bytes());
        };

        write(&mut log);
    }

    PANICKED.store(true, Ordering::Release);
}

pub fn panicked() -> bool {
    PANICKED.load(Ordering::Acquire)
}
