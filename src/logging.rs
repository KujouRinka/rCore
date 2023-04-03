use log::{Level, LevelFilter, Log, Metadata, Record};
use crate::println;

struct SimpleLogger;

impl Log for SimpleLogger {
  fn enabled(&self, _metadata: &Metadata) -> bool {
    true
  }

  fn log(&self, record: &Record) {
    if !self.enabled(record.metadata()) {
      return;
    }
    let color = match record.level() {
      Level::Error => 31,   // red
      Level::Warn => 93,    // yellow
      Level::Info => 34,    // blue
      Level::Debug => 32,   // green
      Level::Trace => 90,   // gray
    };
    println!(
      "\x1b[{}m[{:>5}] {}\x1b[0m",
      color,
      record.level(),
      record.args(),
    );
  }

  fn flush(&self) {}
}

pub fn init(level: Option<LevelFilter>) {
  static LOGGER: SimpleLogger = SimpleLogger;
  log::set_logger(&LOGGER).unwrap();
  log::set_max_level(level.unwrap_or(LevelFilter::Off));
}