use kairoi::{info, AddConsoleHandler, GlobalHandler, Progress, Span, Scope};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let global_handler = GlobalHandler::builder().console_handler().build();

    info!("Hello");

    Span::scope(async |scope: Scope| {
        let data = Span::default().with_name("Hello World".to_string());

        scope.update(data.with_progress(Progress::new(100, 0)));
        for i in 1..=100 {
            if i % 10 == 0 {
                info!("{}/100", i);
            }

            sleep(Duration::from_millis(50)).await;
            scope.update(data.with_progress(Progress::new(100, i)));
        }
    })
    .await;

    global_handler.join();
}
