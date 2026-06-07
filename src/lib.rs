pub mod bot;
pub mod isracard;
pub mod models;

use std::sync::Arc;

use teloxide::types::ChatId;

use bot::BotConfig;
use isracard::IsracardClient;

pub fn load_config() -> BotConfig {
    let allowed_users: Vec<u64> = std::env::var("ALLOWED_USERS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let channel_chat_id = ChatId(
        std::env::var("CHANNEL_CHAT_ID")
            .expect("CHANNEL_CHAT_ID not set")
            .trim()
            .parse::<i64>()
            .expect("CHANNEL_CHAT_ID must be a valid integer"),
    );

    BotConfig {
        allowed_users,
        channel_chat_id,
    }
}

pub fn load_isracard_client() -> Arc<IsracardClient> {
    Arc::new(IsracardClient::new(
        std::env::var("ISRACARD_EMAIL").expect("ISRACARD_EMAIL not set"),
        std::env::var("ISRACARD_PASSWORD").expect("ISRACARD_PASSWORD not set"),
    ))
}
