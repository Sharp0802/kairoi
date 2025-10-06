use kairoi::{info, instrument, AddConsoleHandler, GlobalHandler, Progress, Span};
use std::time::Duration;
use tokio::time::sleep;

#[instrument]
async fn some_instrument() {
    let span = Span::current();

    let data = Span::default().with_name("Hello World".to_string());

    span.update(data.with_progress(Progress::new(100, 0)));
    for i in 1..=100 {
        if i % 10 == 0 {
            info!("{}/100", i);
        }

        sleep(Duration::from_millis(50)).await;
        span.update(data.with_progress(Progress::new(100, i)));
    }
}

#[tokio::main]
async fn main() {
    let global_handler = GlobalHandler::builder().console_handler().build();

    info!("Hello");
    some_instrument().await;

    global_handler.join();
}
