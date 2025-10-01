use crate::error::SendSyncError;
use crate::format::Write;
use crate::format::{DefaultFormatter, FormatterSet, Writer};
use crate::{Event, GlobalHandlerBuilder, Handler, Log, Span, SpanData, SpanId};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::stdout;

pub trait AddConsoleHandler<T> {
    fn console_handler(self) -> T;
}

impl AddConsoleHandler<GlobalHandlerBuilder> for GlobalHandlerBuilder {
    fn console_handler(self) -> Self {
        self.handler(Box::new(ConsoleHandler::new()))
    }
}

pub struct ConsoleHandler {
    span_data: RefCell<HashMap<SpanId, SpanData>>,
    log_queue: RefCell<VecDeque<Log>>,
    cursor_saved: RefCell<bool>,
    formatter: Box<dyn FormatterSet>,
}

impl ConsoleHandler {
    pub fn new() -> Self {
        Self {
            span_data: RefCell::new(HashMap::new()),
            log_queue: RefCell::new(VecDeque::new()),
            cursor_saved: RefCell::new(false),
            formatter: Box::new(DefaultFormatter::new()),
        }
    }
}

impl ConsoleHandler {
    fn print(&self, writer: &mut Writer, span: &Span, depth: usize) -> Result<(), SendSyncError> {
        if depth > 0 {
            let map_ref = self.span_data.borrow();
            let data = match map_ref.get(&span.id()) {
                None => return Ok(()),
                Some(data) => data,
            };

            self.formatter.format(writer, depth, data)?;
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
            Event::SpanBegin { id } => {
                self.span_data
                    .borrow_mut()
                    .insert(id.clone(), SpanData::default());
            }
            Event::Span { id, data } => {
                self.span_data
                    .borrow_mut()
                    .entry(id.clone())
                    .and_modify(|v| *v = data.clone());
            }
            Event::SpanEnd { id } => {
                self.span_data.borrow_mut().remove(id);
            }
        }

        Ok(())
    }

    fn tick(&self, root: &Span) -> Result<(), SendSyncError> {
        let mut lock = stdout().lock();
        let mut writer = Writer::Io(&mut lock);

        if *self.cursor_saved.borrow() {
            write!(writer, "\x1b[u\x1b[J")?;
        }

        while let Some(log) = self.log_queue.borrow_mut().pop_front() {
            let depth = root.find_depth(log.span_id()).unwrap_or(0);

            let span: Option<SpanData> = if depth == 0 {
                None
            } else {
                self.span_data.borrow().get(&log.span_id()).cloned()
            };

            self.formatter.format(&mut writer, depth, &(log, span))?
        }

        {
            write!(writer, "\x1b[s")?;
            self.cursor_saved.replace(true);
            self.print(&mut writer, root, 0)?;
        }

        Ok(())
    }
}
