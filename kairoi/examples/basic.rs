use kairoi::{info, AddConsoleHandler, GlobalHandler};

#[tokio::main]
async fn main() {
    let global_handler = GlobalHandler::builder().console_handler().build();

    info!("Hello");

    drop(global_handler);
}
