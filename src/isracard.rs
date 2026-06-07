use std::time::Duration;

use reqwest::Client;
use tokio::sync::RwLock;

use crate::models::*;

const BASE_URL: &str = "https://www.isracard360.co.il";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct IsracardClient {
    client: Client,
    email: String,
    password: String,
    token: RwLock<Option<String>>,
}

impl IsracardClient {
    pub fn new(email: String, password: String) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            email,
            password,
            token: RwLock::new(None),
        }
    }

    async fn authenticate(&self) -> Result<String, String> {
        let body = AuthRequest {
            captcha: None,
            email: self.email.clone(),
            password_open: self.password.clone(),
        };

        let resp = self
            .client
            .post(format!("{}/api/v2/auth", BASE_URL))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Auth request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Auth failed with status {}", resp.status()));
        }

        let auth: AuthResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse auth response: {e}"))?;

        let token = auth.payload.token;
        *self.token.write().await = Some(token.clone());
        Ok(token)
    }

    async fn get_token(&self) -> Result<String, String> {
        if let Some(token) = self.token.read().await.clone() {
            return Ok(token);
        }
        self.authenticate().await
    }

    pub async fn create_payment_link(
        &self,
        name: String,
        currency: Currency,
        price: u32,
    ) -> Result<String, String> {
        let body = CreateSaleRequest {
            currency: currency.to_string(),
            installments: 1,
            payment_method: "multi".to_string(),
            payment_type: "template".to_string(),
            price,
            product_name: name,
        };

        let token = self.get_token().await?;
        let resp = self
            .client
            .post(format!("{}/api/v2/sales", BASE_URL))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Sales request failed: {e}"))?;

        if resp.status().as_u16() == 401 {
            log::warn!("Token expired, re-authenticating");
            let new_token = self.authenticate().await?;
            let retry = self
                .client
                .post(format!("{}/api/v2/sales", BASE_URL))
                .bearer_auth(&new_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Sales retry failed: {e}"))?;

            if !retry.status().is_success() {
                return Err(format!("Sales failed with status {}", retry.status()));
            }

            let sale: CreateSaleResponse = retry
                .json()
                .await
                .map_err(|e| format!("Failed to parse sales response: {e}"))?;
            return Ok(sale.payload.sale_url);
        }

        if !resp.status().is_success() {
            return Err(format!("Sales failed with status {}", resp.status()));
        }

        let sale: CreateSaleResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse sales response: {e}"))?;
        Ok(sale.payload.sale_url)
    }
}
