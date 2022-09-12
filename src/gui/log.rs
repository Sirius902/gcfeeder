use crossbeam::channel;
use egui::Color32;
use time::{format_description::FormatItem, OffsetDateTime};

pub struct Logger {
    sender: channel::Sender<Message>,
    level: log::LevelFilter,
}

impl Logger {
    pub fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_max_level(self.level);
        log::set_boxed_logger(Box::new(self))?;
        Ok(())
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level().to_level_filter() <= self.level
    }

    fn log(&self, record: &log::Record) {
        const TIMESTAMP_FORMAT: &[FormatItem] = time::macros::format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]"
        );

        if self.enabled(record.metadata()) {
            let timestamp = OffsetDateTime::now_local()
                .expect("Failed to get local timestamp")
                .format(&TIMESTAMP_FORMAT)
                .unwrap();

            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let _ = self.sender.send(Message(
                timestamp,
                record.level(),
                format!("[{}] {}", target, record.args()),
            ));
        }
    }

    fn flush(&self) {}
}

pub struct LoggerBuilder {
    sender: Result<channel::Sender<Message>, BuildError>,
    level: log::LevelFilter,
}

impl LoggerBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Result<Logger, BuildError> {
        self.sender.map(|sender| Logger {
            sender,
            level: self.level,
        })
    }

    #[must_use]
    pub fn sender(mut self, sender: channel::Sender<Message>) -> Self {
        self.sender = Ok(sender);
        self
    }

    #[must_use]
    pub fn with_level(mut self, level: log::LevelFilter) -> Self {
        self.level = level;
        self
    }
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self {
            sender: Err(BuildError::SenderMissing),
            level: log::LevelFilter::Trace,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum BuildError {
    #[error("sender missing")]
    SenderMissing,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Message(String, log::Level, String);

impl Message {
    pub fn draw(&self, ui: &mut egui::Ui) {
        let Message(timestamp, level, message) = self;

        ui.horizontal(|ui| {
            ui.label(timestamp);

            if let Some(color) = Self::level_color(*level) {
                ui.colored_label(color, level.to_string());
            } else {
                ui.label(level.to_string());
            }

            ui.label(message);
        });
    }

    const fn level_color(level: log::Level) -> Option<Color32> {
        use log::Level;
        match level {
            Level::Error => Some(Color32::from_rgb(197, 15, 31)),
            Level::Warn => Some(Color32::from_rgb(193, 156, 0)),
            Level::Info => Some(Color32::from_rgb(58, 150, 221)),
            Level::Debug => Some(Color32::from_rgb(136, 23, 152)),
            Level::Trace => None,
        }
    }
}
