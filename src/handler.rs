use crate::channel::rx;
use crate::{Event, Span};
use crossbeam_channel::TryRecvError;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;
use parking_lot::Mutex;
use crate::error::SendSyncError;

pub trait Handler: Send {
    fn handle(&self, event: &Event) -> Result<(), SendSyncError>;
    fn tick(&self, root: &Span) -> Result<(), SendSyncError>;
}

pub struct GlobalHandlerBuilder {
    handlers: Vec<Box<dyn Handler>>,
}

impl GlobalHandlerBuilder {
    fn new() -> Self {
        Self { handlers: vec![] }
    }

    pub fn handler(mut self, handler: Box<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn build(self) -> GlobalHandler {
        GlobalHandler::new(self.handlers)
    }
}

pub struct GlobalHandler {
    token: Arc<AtomicBool>,
    handle: JoinHandle<Result<(), SendSyncError>>,
}

struct AggregatedError(Mutex<Vec<SendSyncError>>);

impl Debug for AggregatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lock = self.0.lock();

        write!(f, "Aggregated error(s) (total {})", lock.len())?;
        for i in 0..lock.len() {
            writeln!(f, "- {}/{}: {}", i + 1, lock.len(), lock[i])?;
        }

        Ok(())
    }
}

impl Display for AggregatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lock = self.0.lock();

        write!(f, "Aggregated error(s) (total {})", lock.len())?;
        for i in 0..lock.len() {
            writeln!(f, "=== Error Dump {}/{} ===\n{:?}", i + 1, lock.len(), lock[i])?;
        }

        Ok(())
    }
}

impl Error for AggregatedError {}

impl GlobalHandler {
    fn foreach<T, F: Fn(&T) -> Result<(), SendSyncError>>(handlers: &mut Vec<T>, f: F) -> Result<(), AggregatedError> {
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

    fn thread_loop(token: Arc<AtomicBool>, mut handlers: Vec<Box<dyn Handler>>) -> Result<(), SendSyncError> {
        let mut last_update = Instant::now();
        let mut root: Span = Span::new();
        while token.load(Ordering::Acquire) {
            match rx().try_recv() {
                Ok(event) => {
                    Self::foreach(&mut handlers, |handler| handler.handle(&event))?;
                }
                Err(TryRecvError::Disconnected) => {
                    return Err("Channel has been disconnected".into());
                }
                Err(TryRecvError::Empty) => {
                    const FPS: u128 = 10;
                    if last_update.elapsed().as_millis() < (1000 / FPS) {
                        continue;
                    }
                }
            };
            last_update = Instant::now();

            Span::get_root(&mut root);
            Self::foreach(&mut handlers, |handler| handler.tick(&root))?;
        }

        Ok(())
    }

    fn new(handlers: Vec<Box<dyn Handler>>) -> Self {
        let token = Arc::new(AtomicBool::new(true));

        let token_clone = token.clone();
        let handle = thread::spawn(move || -> Result<(), SendSyncError> {
            match Self::thread_loop(token_clone, handlers) {
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

        Self { token, handle }
    }

    pub fn join(self) {
        self.token.store(false, Ordering::Release);
        if let Err(e) = self.handle.join() {
            std::panic::resume_unwind(e);
        }
    }

    pub fn builder() -> GlobalHandlerBuilder {
        GlobalHandlerBuilder::new()
    }
}
