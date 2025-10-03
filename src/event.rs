use crate::channel::tx;
use crate::{Span, SpanRef};
use crossbeam_channel::SendError;
use std::time::SystemTime;

#[derive(Debug, Copy, Clone)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Clone)]
pub struct Log {
    timestamp: SystemTime,
    level: Level,
    message: String,
    span: SpanRef,
}

impl Log {
    fn new(level: Level, message: String) -> Self {
        Self {
            timestamp: SystemTime::now(),
            level,
            message,
            span: Span::current(),
        }
    }

    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn span(&self) -> &SpanRef {
        &self.span
    }
}

pub enum Event {
    Log(Log),
    SpanBegin(SpanRef),
    SpanEnd(SpanRef),
}

impl Event {
    pub fn log(level: Level, message: String) -> Self {
        Event::Log(Log::new(level, message))
    }

    pub fn span_begin(span: SpanRef) -> Self {
        Self::SpanBegin(span)
    }

    pub fn span_end(span: SpanRef) -> Self {
        Self::SpanEnd(span)
    }

    pub fn submit(self) {
        let mut i = 0;
        let mut v = self;
        while let Err(SendError(e)) = tx().send(v) {
            v = e;
            i += 1;
            if i == 5 {
                eprintln!("[kairoi] failed to submit event to event queue");
                break;
            }
        }
    }
}
