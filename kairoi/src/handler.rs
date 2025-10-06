use crate::channel::rx;
use crate::error::SendSyncError;
use crate::{Event, Span, SpanRef};
use crossbeam_channel::TryRecvError;
use parking_lot::Mutex;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant};

pub trait Handler: Send {
    fn handle(&self, event: &Event) -> Result<(), SendSyncError>;
    fn tick(&self, root: &SpanRef) -> Result<(), SendSyncError>;
}

pub struct GlobalHandlerBuilder {
    fps: u32,
    handlers: Vec<Box<dyn Handler>>,
}

impl GlobalHandlerBuilder {
    fn new() -> Self {
        Self {
            fps: 15,
            handlers: vec![],
        }
    }

    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    pub fn handler(mut self, handler: Box<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn build(self) -> GlobalHandler {
        GlobalHandler::new(self.fps, self.handlers)
    }
}

pub struct GlobalHandler {
    token: Arc<AtomicBool>,
    handle: Option<JoinHandle<Result<(), SendSyncError>>>,
}

struct AggregatedError(Mutex<Vec<SendSyncError>>);

impl Debug for AggregatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lock = self.0.lock();

        writeln!(f, "Aggregated error(s) (total {})", lock.len())?;
        for i in 0..lock.len() {
            writeln!(f, "- {}/{}: {}", i + 1, lock.len(), lock[i])?;
        }

        Ok(())
    }
}

impl Display for AggregatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lock = self.0.lock();

        writeln!(f, "Aggregated error(s) (total {})", lock.len())?;
        for i in 0..lock.len() {
            writeln!(
                f,
                "=== Error Dump {}/{} ===\n{:?}",
                i + 1,
                lock.len(),
                lock[i]
            )?;
        }

        Ok(())
    }
}

impl Error for AggregatedError {}

impl GlobalHandler {
    fn foreach<T, F: Fn(&T) -> Result<(), SendSyncError>>(
        handlers: &mut Vec<T>,
        f: F,
    ) -> Result<(), AggregatedError> {
        let mut begin = 0;

        let mut errors: Vec<SendSyncError> = Vec::new();
        while begin < handlers.len() {
            'a: for i in begin..handlers.len() {
                if let Err(e) = f(&handlers[i]) {
                    errors.push(e);
                    handlers.remove(i);
                    #[allow(unused_assignments)]
                    begin = i;
                    break 'a;
                }
            }

            begin = handlers.len();
        }
        if !errors.is_empty() {
            return Err(AggregatedError(Mutex::new(errors)));
        }

        Ok(())
    }

    fn thread_loop(
        fps: u32,
        token: Arc<AtomicBool>,
        mut handlers: Vec<Box<dyn Handler>>,
    ) -> Result<(), SendSyncError> {
        let mut last_update = Instant::now();
        let frame_duration = Duration::from_millis(1000 / fps as u64);

        while token.load(Ordering::Acquire) {
            loop {
                match rx().try_recv() {
                    Ok(event) => {
                        Self::foreach(&mut handlers, |handler| handler.handle(&event))?;
                    }
                    Err(TryRecvError::Disconnected) => {
                        return Err("Channel has been disconnected".into());
                    }
                    Err(TryRecvError::Empty) => {
                        break;
                    }
                };
            }
            if last_update.elapsed() >= frame_duration {
                let root = Span::root();
                Self::foreach(&mut handlers, |handler| handler.tick(&root))?;
                last_update = Instant::now();
            }

            thread::sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    fn new(fps: u32, handlers: Vec<Box<dyn Handler>>) -> Self {
        let token = Arc::new(AtomicBool::new(true));

        let token_clone = token.clone();
        let handle = thread::spawn(move || -> Result<(), SendSyncError> {
            match Self::thread_loop(fps, token_clone, handlers) {
                Ok(_) => Ok(()),
                Err(e) => {
                    const MSG: &str = "[kairoi] logging thread panic; memory consumption with event channel won't be held";
                    #[cfg(debug_assertions)]
                    eprintln!("{MSG}\n{e:?}");
                    #[cfg(not(debug_assertions))]
                    eprintln!("{MSG}\n{e}");
                    Err(e)
                }
            }
        });

        Self {
            token,
            handle: Some(handle),
        }
    }

    pub fn builder() -> GlobalHandlerBuilder {
        GlobalHandlerBuilder::new()
    }
}

impl Drop for GlobalHandler {
    fn drop(&mut self) {
        sleep(Duration::from_millis(100));

        self.token.store(false, Ordering::Release);
        if let Err(e) = self.handle.take().unwrap().join() {
            std::panic::resume_unwind(e);
        }
    }
}
