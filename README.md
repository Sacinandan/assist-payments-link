# Isracard Payment Bot

Telegram bot that generates Isracard payment links on demand. Built with Rust, teloxide, and deployable to AWS Lambda with Telegram webhooks.

![version](https://img.shields.io/badge/version-0.1.0-green.svg)

<img src="https://avatars.githubusercontent.com/u/5430905?s=64&v=4" alt="rust" width="64" height="64">

## Features

- Step-by-step payment link creation (name → currency → price → confirm)
- Inline keyboards for currency selection and confirmation
- User whitelist (Telegram user IDs)
- Payment notifications to a Telegram channel with clickable author profile
- English-only product name validation
- Russian language UI
- Isracard API integration with automatic token refresh on 401

## Architecture

```
src/
├── lib.rs             # Library root (shared code)
├── bot.rs             # Telegram bot handlers, dialogue state machine, keyboards
├── isracard.rs        # Isracard API client (auth + create payment link)
├── models.rs          # Shared types: Currency enum, API request/response structs
├── lambda.rs          # AWS Lambda entry point (webhook handler)
├── polling.rs         # Local development entry point (long polling)
└── setup_webhook.rs   # One-shot utility to register webhook URL with Telegram
```

## Environment Variables

| Variable            | Required | Description                                              |
|---------------------|----------|----------------------------------------------------------|
| `TELOXIDE_TOKEN`    | Yes      | Telegram bot token from [@BotFather](https://t.me/BotFather) |
| `ISRACARD_EMAIL`    | Yes      | Isracard 360 login email                                 |
| `ISRACARD_PASSWORD` | Yes      | Isracard 360 login password                              |
| `ALLOWED_USERS`     | No       | Comma-separated Telegram user IDs (empty = allow all)    |
| `CHANNEL_CHAT_ID`   | Yes      | Telegram channel/group chat ID for payment notifications |

## Local Development

### Prerequisites

- [Rust 1.85+](https://rustup.rs/)
- A `.env` file in the project root (see [.env.example)](.env.example))

### Run with long polling

```bash
cargo run --bin polling
```

## AWS Lambda Deployment

### Prerequisites

1. [AWS CLI](https://aws.amazon.com/cli/) configured with credentials
2. [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html):
   ```bash
   # macOS / Linux
   brew tap cargo-lambda/cargo-lambda
   brew install cargo-lambda

   # or via pip
   pip3 install cargo-lambda

   # or via cargo
   cargo install cargo-lambda
   ```
3. **Zig** (cross-compilation linker, required by cargo-lambda):
   ```bash
   pip3 install ziglang
   ```
   On Windows, Zig installs to Python's `site-packages` and is NOT added to PATH automatically. Add it permanently (PowerShell, one-time):
   ```powershell
   $zigPath = "C:\Users\PHANTEKS\AppData\Local\Programs\Python\Python313\Lib\site-packages\ziglang"
   $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
   [Environment]::SetEnvironmentVariable("Path", "$currentPath;$zigPath", "User")
   ```
   Restart your terminal, then verify: `zig version`

### Step 1: Build for Lambda

```bash
cargo lambda build --release --bin lambda
```

This cross-compiles to `x86_64-unknown-linux-gnu` using rustls (no OpenSSL needed) and produces a `bootstrap` binary (~7.5MB) in `target/lambda/lambda/`.

For ARM64 (Graviton — cheaper, faster cold starts):
```bash
cargo lambda build --release --bin lambda --arm64
```

### Step 2: Deploy to AWS Lambda

```bash
cargo lambda deploy lambda \
  --env-var TELOXIDE_TOKEN=<your-bot-token> \
  --env-var ISRACARD_EMAIL=<email> \
  --env-var ISRACARD_PASSWORD=<password> \
  --env-var ALLOWED_USERS=<user-id-1,user-id-2> \
  --env-var CHANNEL_CHAT_ID=<channel-id> \
  --timeout 30 \
  --memory 128
```

This creates the Lambda function and returns a **Function URL** (or you can configure one).

### Step 3: Enable Function URL

If `cargo lambda deploy` didn't create a public URL, enable it in the AWS Console:

1. Go to **Lambda → Functions → lambda**
2. Click **Configuration → Function URL**
3. Click **Create function URL**
4. Auth type: **NONE** (Telegram sends webhooks without auth headers)
5. Copy the URL (e.g., `https://abc123.lambda-url.eu-west-1.on.aws/`)

Or via AWS CLI:
```bash
aws lambda create-function-url-config \
  --function-name lambda \
  --auth-type NONE

aws lambda add-permission \
  --function-name lambda \
  --statement-id FunctionURLAllowPublicAccess \
  --action lambda:InvokeFunctionUrl \
  --principal "*" \
  --function-url-auth-type NONE
```

### Step 4: Register Webhook with Telegram

```bash
# Using the included utility
cargo run --bin setup_webhook -- https://abc123.lambda-url.eu-west-1.on.aws/

# Or using curl
curl -X POST "https://api.telegram.org/bot<TELOXIDE_TOKEN>/setWebhook" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://abc123.lambda-url.eu-west-1.on.aws/"}'
```

### Step 5: Verify

```bash
curl "https://api.telegram.org/bot<TELOXIDE_TOKEN>/getWebhookInfo"
```

You should see `"url": "https://..."` and `"pending_update_count": 0`.

Open [Telegram Bot](https://t.me/assist_payments_link_bot), send `/start` to your bot, and create a payment link.

## Updating

After code changes:

```bash
cargo lambda build --release --bin lambda
cargo lambda deploy lambda
```

Environment variables persist across deployments unless you pass `--env-var` again.

## Switching Back to Polling

To stop using webhooks and switch back to long polling (e.g., for local dev):

```bash
# Remove the webhook
curl -X POST "https://api.telegram.org/bot<TELOXIDE_TOKEN>/deleteWebhook"

# Run locally
cargo run --bin polling
```

## Important Notes

- **Cold starts**: Rust on Lambda has ~16ms cold starts with 128MB memory — effectively instant.
- **Dialogue state**: Uses in-memory storage. In Lambda, each invocation may run in a different container, so multi-step dialogues rely on Lambda container reuse. For high traffic, consider DynamoDB-backed storage.
- **Timeout**: Set Lambda timeout to at least 30 seconds — the Isracard API auth + sale call chain can take a few seconds.
- **Region**: Deploy to a region close to Telegram's servers (EU West is a good default).
- **Secrets**: For production, use AWS Secrets Manager or SSM Parameter Store instead of Lambda env vars for sensitive values.
- **TLS**: Uses `rustls` instead of `native-tls`/OpenSSL — enables cross-compilation from Windows to Linux without installing OpenSSL headers.
- **reqwest**: Pinned to 0.12.x by teloxide-core 0.13 — do not upgrade to 0.13 until teloxide supports it.

## Cost Estimate

With Rust's minimal memory footprint (128MB) and fast execution (~100ms per invocation):

- **1,000 requests/month**: ~$0.01 (effectively free tier)
- **100,000 requests/month**: ~$0.20
- Lambda Function URLs have no additional cost beyond invocation pricing.

## License

[MIT](LICENSE)