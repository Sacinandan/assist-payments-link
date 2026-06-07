/// One-shot utility to register the Telegram webhook URL.
/// Run locally after deploying the Lambda:
///
///   cargo run --bin setup_webhook -- <LAMBDA_FUNCTION_URL>
///
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let url = std::env::args()
        .nth(1)
        .expect("Usage: setup_webhook <LAMBDA_FUNCTION_URL>");

    let bot = Bot::from_env();

    let webhook_url = url.parse().expect("Invalid URL");
    bot.set_webhook(webhook_url)
        .await
        .expect("Failed to set webhook");

    println!("Webhook set to: {url}");

    let info = bot.get_webhook_info().await.expect("Failed to get webhook info");
    println!("Webhook info: {info:?}");
}
