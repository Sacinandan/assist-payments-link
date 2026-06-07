use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum Currency {
    ILS,
    USD,
    EUR,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Currency::ILS => write!(f, "ILS"),
            Currency::USD => write!(f, "USD"),
            Currency::EUR => write!(f, "EUR"),
        }
    }
}

impl Currency {
    pub fn symbol(&self) -> &'static str {
        match self {
            Currency::ILS => "₪",
            Currency::USD => "$",
            Currency::EUR => "€",
        }
    }

    pub fn flag(&self) -> &'static str {
        match self {
            Currency::ILS => "🇮🇱",
            Currency::USD => "🇺🇸",
            Currency::EUR => "🇪🇺",
        }
    }
}

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ILS" => Ok(Currency::ILS),
            "USD" => Ok(Currency::USD),
            "EUR" => Ok(Currency::EUR),
            other => Err(format!("Unknown currency: {other}")),
        }
    }
}

// --- Auth ---

#[derive(Serialize)]
pub struct AuthRequest {
    pub captcha: Option<String>,
    pub email: String,
    #[serde(rename = "passwordOpen")]
    pub password_open: String,
}

#[derive(Deserialize)]
pub struct AuthResponse {
    pub payload: AuthPayload,
}

#[derive(Deserialize)]
pub struct AuthPayload {
    pub token: String,
}

// --- Sales ---

#[derive(Serialize)]
pub struct CreateSaleRequest {
    pub currency: String,
    pub installments: u32,
    pub payment_method: String,
    #[serde(rename = "paymentType")]
    pub payment_type: String,
    pub price: u32,
    pub product_name: String,
}

#[derive(Deserialize)]
pub struct CreateSaleResponse {
    pub payload: SalePayload,
}

#[derive(Deserialize)]
pub struct SalePayload {
    pub sale_url: String,
}
