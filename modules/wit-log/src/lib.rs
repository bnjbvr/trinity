mod wit {
    wit_bindgen::generate!("log" in "../../wit/log.wit");
    pub use self::log::*;
}

pub use log::*;

/// A log implementation based on calls to the host.
pub struct WitLog {
    enabled: bool,
    max_level: log::LevelFilter,
}

impl WitLog {
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_level: log::LevelFilter::Trace,
        }
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    pub fn set_max_level(&mut self, level: log::LevelFilter) {
        self.max_level = level;
    }
}

impl log::Log for WitLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.enabled && metadata.level().to_level_filter() <= self.max_level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let content = format!("{}", record.args());
        match record.level().to_level_filter() {
            log::LevelFilter::Off => {}
            log::LevelFilter::Error => wit::error(&content),
            log::LevelFilter::Warn => wit::warn(&content),
            log::LevelFilter::Info => wit::info(&content),
            log::LevelFilter::Debug => wit::debug(&content),
            log::LevelFilter::Trace => wit::trace(&content),
        }
    }

    fn flush(&self) {
        // nothing to do here
    }
}
