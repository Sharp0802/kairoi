use crate::channel::rx;
use crate::{Event, Span};
use crossbeam_channel::TryRecvError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::SystemTime;

pub trait Handler: Send {
    fn handle(&self, event: &Event);
    fn tick(&self, root: &Span);
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
    handle: JoinHandle<()>,
}

impl GlobalHandler {
    fn new(handlers: Vec<Box<dyn Handler>>) -> Self {
        let token = Arc::new(AtomicBool::new(true));

        let token_clone = token.clone();
        let handle = thread::spawn(move || {
            let mut last_update = SystemTime::now();
            let mut root: Span = Span::new();
            while token_clone.load(Ordering::Acquire) {
                let now = SystemTime::now();
                match rx().try_recv() {
                    Ok(event) => {
                        for x in &handlers {
                            x.handle(&event);
                        }
                    }
                    Err(TryRecvError::Disconnected) => {
                        return;
                    }
                    Err(TryRecvError::Empty) => {
                        const FPS: u128 = 10;
                        if now.duration_since(last_update).unwrap().as_millis() < (1000 / FPS) {
                            continue;
                        }
                    }
                };
                last_update = now;

                Span::get_root(&mut root);
                for x in &handlers {
                    x.tick(&root);
                }
            }
        });

        Self { token, handle }
    }

    pub fn join(self) {
        self.token.store(false, Ordering::Release);
        self.handle.join().unwrap();
    }

    pub fn builder() -> GlobalHandlerBuilder {
        GlobalHandlerBuilder::new()
    }
}
