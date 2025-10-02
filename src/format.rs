use crate::error::SendSyncError;
use crate::{Level, Log, SpanData};
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
    fn format(&self, to: &mut Writer, depth: usize, value: &T) -> Result<(), SendSyncError>;
}

pub trait FormatterSet: Formatter<(Log, Option<SpanData>)> + Formatter<SpanData> {}

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

    pub fn format_log(
        to: &mut Writer,
        _depth: usize,
        (log, parent): &(Log, Option<SpanData>),
    ) -> Result<(), SendSyncError> {
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

        let args = match &parent {
            None => format_args!("\x1b[90m{t}\x1b[0m {level}\x1b[0m: {message}\n"),
            Some(parent) => format_args!(
                "\x1b[90m{t}\x1b[0m {level}\x1b[0m \x1b[1m{}\x1b[0m: {message}\n",
                parent.name()
            ),
        };

        to.write_fmt(args)?;

        Ok(())
    }

    pub fn format_span(to: &mut Writer, depth: usize, value: &SpanData) -> Result<(), SendSyncError> {
        Self::indent(to, if depth > 0 { depth - 1 } else { depth })?;

        let elapsed = value.timestamp().elapsed().unwrap();

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

impl Formatter<(Log, Option<SpanData>)> for DefaultFormatter {
    fn format(&self, to: &mut Writer, depth: usize, value: &(Log, Option<SpanData>)) -> Result<(), SendSyncError> {
        Self::format_log(to, depth, value)
    }
}

impl Formatter<SpanData> for DefaultFormatter {
    fn format(&self, to: &mut Writer, depth: usize, value: &SpanData) -> Result<(), SendSyncError> {
        Self::format_span(to, depth, value)
    }
}

impl FormatterSet for DefaultFormatter {}
