# Isracard Payment Bot

Telegram bot (Rust/teloxide) that generates Isracard payment links on demand. Deployable as AWS Lambda with Telegram webhook or locally with long polling.

## Stack

- **Language**: Rust (edition 2021, MSRV 1.85)
- **Bot framework**: teloxide 0.17 with `macros` feature
- **Async runtime**: tokio (rt-multi-thread, macros)
- **HTTP client**: reqwest 0.12 with json + rustls-tls (pinned by teloxide-core 0.13 — do NOT upgrade to 0.13 until teloxide supports it)
- **Serialization**: serde + serde_json
- **Config**: dotenvy 0.15 (loads `.env` at startup, local mode only)
- **Logging**: log + pretty_env_logger
- **Lambda**: lambda_http 1.x + lambda_runtime 1.x

## Architecture

```
src/
├── lib.rs             # Library root — load_config(), load_isracard_client()
├── bot.rs             # Telegram bot handlers, dialogue state machine, keyboards
├── isracard.rs        # Isracard API client (auth + create payment link)
├── models.rs          # Shared types: Currency enum, API request/response structs
├── lambda.rs          # AWS Lambda entry point (webhook handler)
├── polling.rs         # Local dev entry point (long polling)
└── setup_webhook.rs   # One-shot utility to register webhook URL with Telegram
```

### Isracard API (from Postman collection)

Two endpoints, both POST, base URL hardcoded as `const BASE_URL` in `isracard.rs`:

1. **Auth** — `POST /api/v2/auth`
   - Body: `{ "captcha": null, "email": "...", "passwordOpen": "..." }`
   - Returns: `{ "payload": { "token": "..." } }`
   - Token cached in `RwLock<Option<String>>`, lazy auth on first call

2. **Create payment link** — `POST /api/v2/sales`
   - Bearer token required
   - Body: `{ "currency": "ILS", "installments": 1, "payment_method": "multi", "paymentType": "template", "price": 100, "product_name": "..." }`
   - Returns: `{ "payload": { "sale_url": "https://..." } }`
   - On 401: re-authenticate and retry once

### Bot UX flow (sequential dialogue with confirmation)

```
/start → Welcome message + [💳 Создать ссылку на оплату] button
  → tap button
  → "Введите название продукта (только английские буквы):"
  → user types name (ASCII-only, validated with is_ascii_text(), max 100 chars)
  → inline keyboard: [🇮🇱 ILS] [🇺🇸 USD] [🇪🇺 EUR]
  → user taps currency
  → "Введите сумму (целое число):"
  → user types price (> 0, ≤ 1,000,000)
  → summary with [✅ Подтвердить] [❌ Отмена]
  → Confirm → "⏳ Генерация ссылки..." → "✅ Ссылка на оплату готова" + link
  → Cancel  → "Отменено." → start keyboard shown again
```

### Dialogue states

```
Start → ReceiveName → ReceiveCurrency { name } → ReceivePrice { name, currency } → Confirm { name, currency, price } → (done → reset to Start)
```

After completion (confirm or cancel), `dialogue.reset()` is called and the start keyboard is shown again for the next payment.

### Channel notifications

On successful payment link creation, a text-only notification is sent to `CHANNEL_CHAT_ID` with:
- Clickable author name via `tg://user?id=...` HTML link
- Product name, currency, price, and payment link
- No avatar/photo — text only

## Environment variables

```
TELOXIDE_TOKEN=<telegram bot token>
ISRACARD_EMAIL=<login email>
ISRACARD_PASSWORD=<login password>
ALLOWED_USERS=<comma-separated telegram user IDs>
CHANNEL_CHAT_ID=<channel chat ID for notifications>
```

## Build prerequisites (Windows → Lambda cross-compilation)

1. Install cargo-lambda: `cargo install cargo-lambda`
2. Install Zig via pip: `pip3 install ziglang`
3. Add Zig to PATH permanently (PowerShell, one-time):
   ```powershell
   $zigPath = "C:\Users\PHANTEKS\AppData\Local\Programs\Python\Python313\Lib\site-packages\ziglang"
   $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
   [Environment]::SetEnvironmentVariable("Path", "$currentPath;$zigPath", "User")
   ```
4. Restart terminal, verify: `zig version`

## Commands

```bash
# Local development (long polling)
cargo run --bin polling

# Build for AWS Lambda (requires cargo-lambda + zig in PATH)
cargo lambda build --release --bin lambda

# Deploy to AWS Lambda
cargo lambda deploy lambda --env-var TELOXIDE_TOKEN=... ...

# Register webhook with Telegram
cargo run --bin setup_webhook -- <LAMBDA_FUNCTION_URL>

# Windows release build
cargo build --release --target x86_64-pc-windows-msvc

# Run without .env file (PowerShell)
$env:TELOXIDE_TOKEN="..."; $env:ISRACARD_EMAIL="..."; $env:ISRACARD_PASSWORD="..."; $env:ALLOWED_USERS="..."; $env:CHANNEL_CHAT_ID="..."; cargo run --bin polling
```

## Conventions

- All API calls go through `isracard.rs` — bot handlers never make HTTP requests directly
- Token refresh: re-authenticate if a sales call returns 401, retry once
- Currency is an enum (`ILS`, `USD`, `EUR`) — never a raw string in bot logic
- Errors are reported back to the user in chat; never panic on API failures
- Keep bot handlers thin: validate input, delegate to isracard client, format response
- UI language: Russian. Product name input: English only (ASCII validated)
- Channel notifications include clickable author profile link (`tg://user?id=...`)
- All messages use HTML parse mode (not MarkdownV2) for consistency
- User input is HTML-escaped via `escape_html()` before embedding in messages
- Whitelist filter runs before dialogue entry — unauthorized users are silently ignored
- `LinkPreviewOptions` must be constructed with all 5 fields (no Default impl in teloxide 0.17)
- `answer_callback_query` takes owned `q.id.clone()`, not `&q.id`
- `reqwest` must stay at 0.12.x — teloxide-core 0.13 pins it; upgrading causes duplicate deps
- TLS: uses `rustls` (not `native-tls`/OpenSSL) — enables cross-compilation to Linux without OpenSSL headers
- Lambda cross-compile from Windows requires Zig in PATH (installed via `pip3 install ziglang`, add to PATH permanently via `[Environment]::SetEnvironmentVariable`)

## Dependency version constraints

| Crate | Version | Constraint |
|---|---|---|
| teloxide | 0.17 | Latest stable |
| reqwest | 0.12 | Pinned by teloxide-core 0.13; uses rustls-tls, NOT native-tls |
| lambda_http | 1.x | `Body` enum is non-exhaustive — match arms need wildcard `_` |
| lambda_runtime | 1.x | Latest stable |
| tokio | 1.x | Latest stable |
| serde | 1.x | Latest stable |
| dotenvy | 0.15 | Latest stable |
| pretty_env_logger | 0.5 | Latest stable |
