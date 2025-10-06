use crate::error::SendSyncError;
use crate::format::Write;
use crate::format::{DefaultFormatter, FormatterSet, Writer};
use crate::{Event, GlobalHandlerBuilder, Handler, Log, SpanRef};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::stdout;
use std::ops::Deref;
use std::sync::Arc;

pub trait AddConsoleHandler<T> {
    fn console_handler(self) -> T;
}

impl AddConsoleHandler<GlobalHandlerBuilder> for GlobalHandlerBuilder {
    fn console_handler(self) -> Self {
        self.handler(Box::new(ConsoleHandler::new()))
    }
}

pub struct ConsoleHandler {
    log_queue: RefCell<VecDeque<Arc<Log>>>,
    cursor_saved: RefCell<bool>,
    formatter: Box<dyn FormatterSet>,
}

impl ConsoleHandler {
    pub fn new() -> Self {
        Self {
            log_queue: RefCell::new(VecDeque::new()),
            cursor_saved: RefCell::new(false),
            formatter: Box::new(DefaultFormatter::new()),
        }
    }
}

impl ConsoleHandler {
    fn print(
        &self,
        writer: &mut Writer,
        span: &SpanRef,
        depth: usize,
    ) -> Result<(), SendSyncError> {
        if depth > 0 {
            self.formatter.format(writer, span)?;
        }

        for child in span.children() {
            if let Err(e) = self.print(writer, &child, depth + 1) {
                return Err(e);
            }
        }

        Ok(())
    }
}

impl Handler for ConsoleHandler {
    fn handle(&self, event: &Event) -> Result<(), SendSyncError> {
        match event {
            Event::Log(log) => self.log_queue.borrow_mut().push_back(log.clone()),
            Event::SpanBegin(_) => {}
            Event::SpanEnd(_) => {}
        }

        Ok(())
    }

    fn tick(&self, root: &SpanRef) -> Result<(), SendSyncError> {
        let mut lock = stdout().lock();

        let mut writer = Writer::Io(&mut lock);

        if *self.cursor_saved.borrow() {
            write!(writer, "\x1b[u\x1b[J")?;
        }

        while let Some(log) = self.log_queue.borrow_mut().pop_front() {
            self.formatter.format(&mut writer, log.deref())?
        }

        {
            write!(writer, "\x1b[s")?;
            self.cursor_saved.replace(true);
            self.print(&mut writer, root, 0)?;
        }

        drop(writer);

        std::io::Write::flush(&mut lock)?;

        Ok(())
    }
}
