/* #################### Start of borrowed code (See credits.md) #################### */
use crate::colored::Colorize;
use log::{Level, LevelFilter, Metadata, Record};

pub struct OurLogger;
impl log::Log for OurLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, rec: &Record) {
        if self.enabled(rec.metadata()) {
            match rec.level() {
                Level::Error => eprintln!("{} {}", "error:".red().bold(), rec.args()),
                Level::Warn => eprintln!("{} {}", "warn:".yellow().bold(), rec.args()),
                Level::Info => eprintln!("{} {}", "info:".yellow().bold(), rec.args()),
                Level::Debug => eprintln!("{} {}", "debug:".bright_black().bold(), rec.args()),
                Level::Trace => eprintln!("{} {}", "trace:".blue().bold(), rec.args()),
            }
        }
    }
    fn flush(&self) {}
}
/* ############################## End of borrowed code ############################## */

pub fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&crate::logger::OurLogger)?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}