use crossbeam::channel;
use egui::Color32;
use time::{format_description::FormatItem, OffsetDateTime};

use crate::gui::{DEBUG_COLOR, ERROR_COLOR, INFO_COLOR, WARN_COLOR};

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
                .unwrap_or_else(|_| OffsetDateTime::now_utc())
                .format(&TIMESTAMP_FORMAT)
                .unwrap();

            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            #[cfg(feature = "no-log-spam")]
            if target.starts_with("wgpu") || target.starts_with("naga") {
                return;
            }

            let _ = self.sender.send(Message {
                timestamp,
                level: record.level(),
                message: format!("[{}] {}", target, record.args()),
            });
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

#[derive(Clone)]
pub struct Message {
    timestamp: String,
    level: log::Level,
    message: String,
}

impl Message {
    pub fn draw(&self, ui: &mut egui::Ui) {
        let Message {
            timestamp,
            level,
            message,
        } = self;

        ui.label(timestamp);

        if let Some(color) = Self::level_color(*level) {
            ui.colored_label(color, level.to_string());
        } else {
            ui.label(level.to_string());
        }

        ui.label(message);
    }

    const fn level_color(level: log::Level) -> Option<Color32> {
        use log::Level;
        match level {
            Level::Error => Some(ERROR_COLOR),
            Level::Warn => Some(WARN_COLOR),
            Level::Info => Some(INFO_COLOR),
            Level::Debug => Some(DEBUG_COLOR),
            Level::Trace => None,
        }
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level && self.message == other.message
    }
}

impl Eq for Message {}
