# Skill: Isracard Payment API

## Base URL

Hardcoded as `const BASE_URL: &str = "https://www.isracard360.co.il"` in `src/isracard.rs`.

## Authentication

**POST** `{BASE_URL}/api/v2/auth`

```json
{
  "captcha": null,
  "email": "{{ISRACARD_EMAIL}}",
  "passwordOpen": "{{ISRACARD_PASSWORD}}"
}
```

Response:
```json
{ "payload": { "token": "jwt-token-string" } }
```

Use the token as `Authorization: Bearer <token>` on all subsequent requests.

### Token lifecycle
- Authenticate lazily on first API call, cache token in `RwLock<Option<String>>`
- On 401 from any endpoint, re-authenticate and retry once
- Credentials come from env vars (`ISRACARD_EMAIL`, `ISRACARD_PASSWORD`)
- HTTP client created with `Client::builder().timeout(Duration::from_secs(10))`

## Create Payment Link

**POST** `{BASE_URL}/api/v2/sales` (Bearer token required)

```json
{
  "currency": "ILS",
  "installments": 1,
  "payment_method": "multi",
  "paymentType": "template",
  "price": 100,
  "product_name": "My Product"
}
```

Response:
```json
{ "payload": { "sale_url": "https://live.payme.io/sale/template/..." } }
```

### Field mapping from bot input

| Bot field | API field        | Notes                              |
|-----------|------------------|------------------------------------|
| Name      | `product_name`   | English-only ASCII text from user  |
| Currency  | `currency`       | Enum: `ILS`, `USD`, `EUR`          |
| Price     | `price`          | u32, whole number, > 0, ≤ 1000000  |
| —         | `installments`   | Hardcoded to `1`                   |
| —         | `payment_method` | Hardcoded to `"multi"`             |
| —         | `paymentType`    | Hardcoded to `"template"`          |

### Result
Return `payload.sale_url` to the user — this is the payment link.

## Rust implementation (`src/isracard.rs`)

```rust
pub struct IsracardClient {
    client: reqwest::Client,    // with 10s timeout
    email: String,
    password: String,
    token: RwLock<Option<String>>,
}

impl IsracardClient {
    pub fn new(email: String, password: String) -> Self;
    async fn authenticate(&self) -> Result<String, String>;
    async fn get_token(&self) -> Result<String, String>;  // lazy: auth only if no cached token
    pub async fn create_payment_link(&self, name: String, currency: Currency, price: u32) -> Result<String, String>;
}
```

### 401 retry pattern
```rust
let resp = self.client.post(url).bearer_auth(&token).json(&body).send().await?;
if resp.status().as_u16() == 401 {
    let new_token = self.authenticate().await?;
    let retry = self.client.post(url).bearer_auth(&new_token).json(&body).send().await?;
    // parse retry response
}
```

## Rust types (`src/models.rs`)

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Currency { ILS, USD, EUR }

impl Currency {
    pub fn symbol(&self) -> &'static str;  // ₪, $, €
    pub fn flag(&self) -> &'static str;    // 🇮🇱, 🇺🇸, 🇪🇺
}
impl fmt::Display for Currency { ... }     // "ILS", "USD", "EUR"
impl FromStr for Currency { ... }          // parse from callback data

#[derive(Serialize)]
pub struct AuthRequest { captcha: Option<String>, email: String, #[serde(rename = "passwordOpen")] password_open: String }

#[derive(Serialize)]
pub struct CreateSaleRequest { currency: String, installments: u32, payment_method: String, #[serde(rename = "paymentType")] payment_type: String, price: u32, product_name: String }

// Responses: AuthResponse { payload: AuthPayload { token } }, CreateSaleResponse { payload: SalePayload { sale_url } }
```

### Serde rename gotchas
- `passwordOpen` — camelCase in API, `password_open` in Rust → `#[serde(rename = "passwordOpen")]`
- `paymentType` — camelCase in API, `payment_type` in Rust → `#[serde(rename = "paymentType")]`

## Error handling
- Network errors → log, tell user "API unavailable, try again later"
- 401 → re-auth and retry once, if still fails report to user
- 4xx/5xx → log response body, tell user "Failed to create link"
- Never expose raw API errors or tokens to the Telegram chat
