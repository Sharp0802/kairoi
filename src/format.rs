use crate::error::SendSyncError;
use crate::{Level, Log, SpanRef};
use chrono::{DateTime, Local};
use std::fmt::Arguments;

pub trait Write {
    fn write_fmt(&mut self, fmt: Arguments) -> Result<(), SendSyncError>;
}

pub enum Writer<'a> {
    Io(&'a mut dyn std::io::Write),
    Fmt(&'a mut dyn std::fmt::Write),
}

impl<'a> Write for Writer<'a> {
    fn write_fmt(&mut self, fmt: Arguments) -> Result<(), SendSyncError> {
        match self {
            Writer::Io(w) => w.write_fmt(fmt)?,
            Writer::Fmt(w) => w.write_fmt(fmt)?,
        }

        Ok(())
    }
}

pub trait Formatter<T>: Send + Sync {
    fn format(&self, to: &mut Writer, value: &T) -> Result<(), SendSyncError>;
}

pub trait FormatterSet: Formatter<Log> + Formatter<SpanRef> {}

pub struct DefaultFormatter;

impl DefaultFormatter {
    pub fn new() -> Self {
        Self {}
    }

    fn indent(to: &mut Writer, depth: usize) -> Result<(), SendSyncError> {
        for _ in 0..depth {
            write!(to, "  ")?;
        }

        Ok(())
    }

    pub fn format_log(to: &mut Writer, log: &Log) -> Result<(), SendSyncError> {
        let t: DateTime<Local> = log.timestamp().into();
        let t = t.format("%+");

        // use of 3-bit color sequence
        let level = match log.level() {
            Level::Error => "\x1b[31mERROR", // red
            Level::Warn => "\x1b[33m WARN",  // yellow
            Level::Info => "\x1b[32m INFO",  // green
            Level::Debug => "\x1b[36mDEBUG", // cyan
            Level::Trace => "\x1b[35mTRACE", // magenta
        };

        let message = log.message();

        if let Some(parent) = &log.span().parent() {
            to.write_fmt(format_args!(
                "\x1b[90m{t}\x1b[0m {level}\x1b[0m \x1b[1m{}\x1b[0m: {message}\n",
                parent.value().name()
            ))?;
        } else {
            to.write_fmt(format_args!(
                "\x1b[90m{t}\x1b[0m {level}\x1b[0m: {message}\n"
            ))?;
        };

        Ok(())
    }

    pub fn format_span(to: &mut Writer, value: &SpanRef) -> Result<(), SendSyncError> {
        let depth = if value.depth() > 0 {
            value.depth() - 1
        } else {
            value.depth()
        };
        Self::indent(to, depth)?;

        let value = value.value();
        let elapsed = value.timestamp().elapsed().unwrap_or_default();

        let ch = vec!['⠖', '⠲', '⠴', '⠦'][((elapsed.as_millis() / 100) % 4) as usize];

        let mut t = elapsed.as_secs_f32();
        let mut suffix = 's';
        if t > 60_f32 {
            t /= 60_f32;
            suffix = 'm';
        }
        if t > 60_f32 {
            t /= 60_f32;
            suffix = 'h';
        }

        let args = match value.progress() {
            None => format_args!("{ch} {} [{t:.1}{suffix}]\n", value.name()),
            Some(p) => format_args!(
                "{ch} {} ({}/{}) [{t:.1}{suffix}]\n",
                value.name(),
                p.progress(),
                p.total()
            ),
        };

        to.write_fmt(args)?;

        Ok(())
    }
}

impl Formatter<Log> for DefaultFormatter {
    fn format(&self, to: &mut Writer, value: &Log) -> Result<(), SendSyncError> {
        Self::format_log(to, value)
    }
}

impl Formatter<SpanRef> for DefaultFormatter {
    fn format(&self, to: &mut Writer, value: &SpanRef) -> Result<(), SendSyncError> {
        Self::format_span(to, value)
    }
}

impl FormatterSet for DefaultFormatter {}
