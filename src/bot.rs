use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ChatId, UserId};

use crate::isracard::IsracardClient;
use crate::models::Currency;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone)]
pub struct BotConfig {
    pub allowed_users: Vec<u64>,
    pub channel_chat_id: ChatId,
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveName,
    ReceiveCurrency {
        name: String,
    },
    ReceivePrice {
        name: String,
        currency: Currency,
    },
    Confirm {
        name: String,
        currency: Currency,
        price: u32,
    },
}

fn is_allowed(user_id: UserId, config: &BotConfig) -> bool {
    config.allowed_users.is_empty() || config.allowed_users.contains(&user_id.0)
}

const MAX_NAME_LENGTH: usize = 100;
const MAX_PRICE: u32 = 1_000_000;

fn is_ascii_text(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ')
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn no_preview() -> teloxide::types::LinkPreviewOptions {
    teloxide::types::LinkPreviewOptions {
        is_disabled: true,
        url: None,
        prefer_small_media: false,
        prefer_large_media: false,
        show_above_text: false,
    }
}

fn start_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "💳 Создать ссылку на оплату",
        "create_link",
    )]])
}

fn currency_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("🇮🇱 ILS", "ILS"),
        InlineKeyboardButton::callback("🇺🇸 USD", "USD"),
        InlineKeyboardButton::callback("🇪🇺 EUR", "EUR"),
    ]])
}

fn confirm_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ Подтвердить", "confirm"),
        InlineKeyboardButton::callback("❌ Отмена", "cancel"),
    ]])
}

pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let message_handler = Update::filter_message()
        .filter_map(|msg: Message, config: BotConfig| {
            msg.from.as_ref().filter(|u| is_allowed(u.id, &config)).map(|_| ())
        })
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::Start].endpoint(handle_start))
        .branch(dptree::case![State::ReceiveName].endpoint(receive_name))
        .branch(
            dptree::case![State::ReceivePrice { name, currency }].endpoint(receive_price),
        );

    let callback_handler = Update::filter_callback_query()
        .filter_map(|q: CallbackQuery, config: BotConfig| {
            is_allowed(q.from.id, &config).then_some(())
        })
        .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
        .branch(dptree::case![State::Start].endpoint(handle_start_callback))
        .branch(
            dptree::case![State::ReceiveCurrency { name }].endpoint(receive_currency_callback),
        )
        .branch(
            dptree::case![State::Confirm {
                name,
                currency,
                price
            }]
            .endpoint(handle_confirm_callback),
        );

    dptree::entry()
        .branch(message_handler)
        .branch(callback_handler)
}

async fn handle_start(bot: Bot, _dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Добро пожаловать! Нажмите кнопку, чтобы создать ссылку на оплату.",
    )
    .reply_markup(start_keyboard())
    .await?;
    Ok(())
}

async fn handle_start_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await?;
    if q.data.as_deref() != Some("create_link") {
        return Ok(());
    }
    if let Some(msg) = &q.message {
        bot.send_message(msg.chat().id, "Введите название продукта (только английские буквы):")
            .await?;
    }
    dialogue.update(State::ReceiveName).await?;
    Ok(())
}

async fn receive_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let name = msg.text().unwrap_or_default().trim().to_string();
    if name.is_empty() {
        bot.send_message(msg.chat.id, "Название не может быть пустым. Введите название продукта:")
            .await?;
        return Ok(());
    }
    if !is_ascii_text(&name) {
        bot.send_message(msg.chat.id, "⚠️ Название должно содержать только английские буквы.\nПожалуйста, введите название на английском:")
            .await?;
        return Ok(());
    }
    if name.len() > MAX_NAME_LENGTH {
        bot.send_message(msg.chat.id, format!("⚠️ Название слишком длинное (макс. {MAX_NAME_LENGTH} символов). Попробуйте короче:"))
            .await?;
        return Ok(());
    }
    bot.send_message(msg.chat.id, "Выберите валюту:")
        .reply_markup(currency_keyboard())
        .await?;
    dialogue
        .update(State::ReceiveCurrency { name })
        .await?;
    Ok(())
}

async fn receive_currency_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    name: String,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await?;
    let Some(data) = &q.data else { return Ok(()) };
    let currency: Currency = data.parse().map_err(|e: String| e)?;

    if let Some(msg) = &q.message {
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            format!("Валюта: {currency}"),
        )
        .await?;
        bot.send_message(msg.chat().id, "Введите сумму (целое число):")
            .await?;
    }
    dialogue
        .update(State::ReceivePrice { name, currency })
        .await?;
    Ok(())
}

async fn receive_price(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    (name, currency): (String, Currency),
) -> HandlerResult {
    let text = msg.text().unwrap_or_default().trim();
    let price: u32 = match text.parse() {
        Ok(p) if p > 0 && p <= MAX_PRICE => p,
        Ok(p) if p > MAX_PRICE => {
            bot.send_message(msg.chat.id, format!("⚠️ Сумма слишком большая (макс. {MAX_PRICE}). Введите сумму:"))
                .await?;
            return Ok(());
        }
        _ => {
            bot.send_message(msg.chat.id, "Неверная сумма. Введите целое число больше 0:")
                .await?;
            return Ok(());
        }
    };

    let symbol = currency.symbol();
    let flag = currency.flag();
    let safe_name = escape_html(&name);
    let summary = format!(
        "<b>📋 Сводка платежа</b>\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         📦  <b>{safe_name}</b>\n\
         {flag}  {currency}\n\
         💰  <b>{symbol}{price}</b>\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         Подтвердите для создания ссылки"
    );
    bot.send_message(msg.chat.id, summary)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(confirm_keyboard())
        .await?;
    dialogue
        .update(State::Confirm {
            name,
            currency,
            price,
        })
        .await?;
    Ok(())
}

async fn handle_confirm_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    (name, currency, price): (String, Currency, u32),
    client: Arc<IsracardClient>,
    config: BotConfig,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await?;
    let Some(msg) = &q.message else { return Ok(()) };
    let chat_id = msg.chat().id;
    let msg_id = msg.id();

    match q.data.as_deref() {
        Some("confirm") => {
            bot.edit_message_text(chat_id, msg_id, "⏳ Генерация ссылки...")
                .await?;

            let symbol = currency.symbol();
            let flag = currency.flag();

            let safe_name = escape_html(&name);

            match client.create_payment_link(name.clone(), currency.clone(), price).await {
                Ok(url) => {
                    let dm_msg = format!(
                        "<b>✅ Ссылка на оплату готова</b>\n\
                         ━━━━━━━━━━━━━━━━━━━━\n\
                         📦  {safe_name}\n\
                         {flag}  {currency}\n\
                         💰  <b>{symbol}{price}</b>\n\
                         ━━━━━━━━━━━━━━━━━━━━\n\
                         🔗  <a href=\"{url}\">Открыть страницу оплаты</a>"
                    );
                    bot.send_message(chat_id, dm_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .link_preview_options(no_preview())
                        .await?;

                    let user = &q.from;
                    let author_name = match &user.last_name {
                        Some(last) => format!("{} {last}", user.first_name),
                        None => user.first_name.clone(),
                    };
                    let safe_author = escape_html(&author_name);
                    let user_id = user.id.0;
                    let channel_msg = format!(
                        "💳  <b><a href=\"tg://user?id={user_id}\">{safe_author}</a></b>\n\
                         ━━━━━━━━━━━━━━━━━━━━\n\
                         📦  <b>{safe_name}</b>\n\
                         {flag}  {currency}\n\
                         💰  <b>{symbol}{price}</b>\n\
                         ━━━━━━━━━━━━━━━━━━━━\n\
                         🔗  <a href=\"{url}\">Ссылка на оплату</a>"
                    );
                    if let Err(e) = bot
                        .send_message(config.channel_chat_id, channel_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .link_preview_options(no_preview())
                        .await
                    {
                        log::error!("Failed to send to channel: {e}");
                    }
                }
                Err(e) => {
                    log::error!("Failed to create payment link: {e}");
                    bot.send_message(chat_id, "❌ Не удалось создать ссылку. Попробуйте позже.")
                        .await?;
                }
            }
        }
        _ => {
            bot.edit_message_text(chat_id, msg_id, "Отменено.").await?;
        }
    }

    dialogue.reset().await?;

    // Show the start button again for the next payment
    bot.send_message(chat_id, "Нажмите кнопку, чтобы создать новую ссылку на оплату.")
        .reply_markup(start_keyboard())
        .await?;

    Ok(())
}
