# Skill: Teloxide Telegram Bot Development

## teloxide 0.17 — Patterns Used in This Project

### API gotchas (teloxide 0.17 / teloxide-core 0.13)

- `bot.answer_callback_query(q.id.clone())` — takes owned `CallbackQueryId`, NOT `&q.id`
- `bot.send_message().link_preview_options(...)` — NOT `disable_web_page_preview(true)` (removed in 0.17)
- `LinkPreviewOptions` has NO `Default` impl — must construct all 5 fields explicitly:
  ```rust
  teloxide::types::LinkPreviewOptions {
      is_disabled: true,   // bool, not Option<bool>
      url: None,
      prefer_small_media: false,
      prefer_large_media: false,
      show_above_text: false,
  }
  ```
- `InputFile::file_id(id)` takes owned `FileId`, not `&FileId` — use `.clone()`
- `UpdateHandler` type lives at `teloxide::dispatching::UpdateHandler`, not in prelude
- teloxide-core 0.13 pins `reqwest ^0.12` — do NOT upgrade reqwest to 0.13 (causes duplicate deps)

### Bot initialization (two modes)

**Polling (local dev)** — uses `Dispatcher`:
```rust
dotenvy::dotenv().ok();
let bot = Bot::from_env(); // reads TELOXIDE_TOKEN
Dispatcher::builder(bot, bot::schema())
    .dependencies(dptree::deps![storage, client, config])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
```

**Lambda (production webhook)** — manual dispatch via `DependencyMap`:
```rust
let handler = Arc::new(bot::schema());
let mut deps = DependencyMap::new();
deps.insert(bot); deps.insert(storage); deps.insert(client); deps.insert(config); deps.insert(me);
let deps = Arc::new(deps);

// Per request:
let update: Update = serde_json::from_str(&body)?;
let mut dep_map = (*deps).as_ref().clone();
dep_map.insert(update);
match handler.dispatch(dep_map).await {
    ControlFlow::Break(Ok(())) => {}
    ControlFlow::Break(Err(err)) => log::error!("Handler error: {:?}", err),
    ControlFlow::Continue(_) => log::warn!("Update was not handled"),
}
```

**Important**: Must insert `bot.get_me()` result into DependencyMap for Lambda mode. `Dispatcher` does this automatically; manual dispatch does not.

### Dialogue state machine

```rust
type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveName,
    ReceiveCurrency { name: String },
    ReceivePrice { name: String, currency: Currency },
    Confirm { name: String, currency: Currency, price: u32 },
}
```

After flow completion (confirm or cancel), call `dialogue.reset().await?` and show the start keyboard again so the user can create the next payment without typing `/start`.

### Dispatcher schema with whitelist filter

```rust
pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let message_handler = Update::filter_message()
        .filter_map(|msg: Message, config: BotConfig| {
            msg.from.as_ref().filter(|u| is_allowed(u.id, &config)).map(|_| ())
        })
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::Start].endpoint(handle_start))
        .branch(dptree::case![State::ReceiveName].endpoint(receive_name))
        .branch(dptree::case![State::ReceivePrice { name, currency }].endpoint(receive_price));

    let callback_handler = Update::filter_callback_query()
        .filter_map(|q: CallbackQuery, config: BotConfig| {
            is_allowed(q.from.id, &config).then_some(())
        })
        .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
        .branch(dptree::case![State::Start].endpoint(handle_start_callback))
        .branch(dptree::case![State::ReceiveCurrency { name }].endpoint(receive_currency_callback))
        .branch(dptree::case![State::Confirm { name, currency, price }].endpoint(handle_confirm_callback));

    dptree::entry()
        .branch(message_handler)
        .branch(callback_handler)
}
```

Key: `filter_map` runs BEFORE `enter_dialogue` — unauthorized users are silently dropped.

### Injecting extra dependencies into handlers

Handler functions receive deps via positional args. teloxide auto-injects from DependencyMap:
```rust
async fn handle_confirm_callback(
    bot: Bot,                                         // from deps
    dialogue: MyDialogue,                             // from dialogue system
    q: CallbackQuery,                                 // from update filter
    (name, currency, price): (String, Currency, u32), // from State variant
    client: Arc<IsracardClient>,                      // from deps
    config: BotConfig,                                // from deps
) -> HandlerResult { ... }
```

### Inline keyboards

```rust
fn currency_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("🇮🇱 ILS", "ILS"),
        InlineKeyboardButton::callback("🇺🇸 USD", "USD"),
        InlineKeyboardButton::callback("🇪🇺 EUR", "EUR"),
    ]])
}
```

### Callback query handling

```rust
bot.answer_callback_query(q.id.clone()).await?;  // MUST acknowledge
let data = q.data.as_deref();                     // Option<&str>
if let Some(msg) = &q.message {
    bot.edit_message_text(msg.chat().id, msg.id(), "new text").await?;
}
```

### HTML-formatted messages with user input escaping

```rust
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// Always escape user-provided text before embedding in HTML messages
let safe_name = escape_html(&name);
bot.send_message(chat_id, format!("<b>Product:</b> {safe_name}"))
    .parse_mode(teloxide::types::ParseMode::Html)
    .link_preview_options(no_preview())
    .await?;
```

### Channel notifications with clickable user profile

```rust
let channel_msg = format!(
    "💳  <b><a href=\"tg://user?id={user_id}\">{author_name}</a></b>\n..."
);
bot.send_message(config.channel_chat_id, channel_msg)
    .parse_mode(teloxide::types::ParseMode::Html)
    .link_preview_options(no_preview())
    .await?;
```

### Input validation constants

```rust
const MAX_NAME_LENGTH: usize = 100;
const MAX_PRICE: u32 = 1_000_000;

fn is_ascii_text(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ')
}
```

### Telegram Bot API limitations

- Cannot send messages as another user (bot is always the author)
- Cannot embed inline images in text messages
- Profile photos via `send_photo` always render full-size (no thumbnail/round option)
- `send_photo` supports HTML captions with clickable links
- Stickers and video notes do NOT support captions
- `tg://user?id=...` links are the only way to create clickable user mentions

### lambda_http 1.x changes

- `lambda_http::Body` enum is marked `#[non_exhaustive]` — match arms MUST include a wildcard `_ => { ... }` fallback
- API otherwise compatible with 0.13

### Cross-compilation (Windows → Lambda/Linux)

- TLS must use `rustls`, NOT `native-tls` — OpenSSL headers unavailable for Linux target on Windows
- Cargo.toml features:
  ```toml
  teloxide = { version = "0.17", default-features = false, features = ["macros", "ctrlc_handler", "rustls", "throttle"] }
  reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
  ```
- Zig required as cross-linker: `pip3 install ziglang`, then add to PATH permanently:
  ```powershell
  $zigPath = "C:\Users\PHANTEKS\AppData\Local\Programs\Python\Python313\Lib\site-packages\ziglang"
  [Environment]::SetEnvironmentVariable("Path", "$([Environment]::GetEnvironmentVariable('Path','User'));$zigPath", "User")
  ```
- Build: `cargo lambda build --release --bin lambda` → produces `target/lambda/lambda/bootstrap` (~7.5MB)
