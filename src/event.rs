use crate::span::{Span, SpanId};
use crate::SpanData;
use std::time::SystemTime;
use crossbeam_channel::SendError;
use crate::channel::tx;

#[derive(Debug, Copy, Clone)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone)]
pub struct Log {
    timestamp: SystemTime,
    level: Level,
    message: String,
    span_id: SpanId,
}

impl Log {
    fn new(level: Level, message: String) -> Self {
        Self {
            timestamp: SystemTime::now(),
            level,
            message,
            span_id: Span::current(),
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

    pub fn span_id(&self) -> SpanId {
        self.span_id
    }
}

#[derive(Debug)]
pub enum Event {
    Log(Log),
    SpanBegin {
        id: SpanId,
    },
    Span {
        id: SpanId,
        data: SpanData,
    },
    SpanEnd {
        id: SpanId,
    }
}

impl Event {
    pub fn log(level: Level, message: String) -> Self {
        Event::Log(Log::new(level, message))
    }

    pub fn span_begin(id: SpanId) -> Self {
        Self::SpanBegin { id }
    }

    pub fn span(id: SpanId, data: SpanData) -> Self {
        Self::Span { id, data }
    }

    pub fn span_end(id: SpanId) -> Self {
        Self::SpanEnd { id }
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
