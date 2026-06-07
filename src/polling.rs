use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

use isracard_payment::bot::{self, State};
use isracard_payment::{load_config, load_isracard_client};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting Isracard payment bot (polling)...");

    let bot = Bot::from_env();

    Dispatcher::builder(bot, bot::schema())
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            load_isracard_client(),
            load_config()
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
