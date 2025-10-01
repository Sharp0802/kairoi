use crate::span::{Span, SpanId};
use crate::SpanData;
use std::time::SystemTime;
use crate::channel::tx;

#[derive(Debug, Copy, Clone)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug)]
pub enum Event {
    Log {
        timestamp: SystemTime,
        level: Level,
        message: String,
        span_id: SpanId,
    },
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
        Self::Log {
            timestamp: SystemTime::now(),
            level,
            message,
            span_id: Span::current(),
        }
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
        tx().send(self).unwrap();
    }
}
