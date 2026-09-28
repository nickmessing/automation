//! A client for the Frankfurter v2 API (https://frankfurter.dev).

use std::time::Duration;

use chrono::NaiveDate;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::Error;

pub const DEFAULT_API: &str = "https://api.frankfurter.dev/v2";

/// One exchange rate, exactly as the API returns it (and as it is saved to disk).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    pub date: NaiveDate,
    pub base: String,
    pub quote: String,
    pub rate: f64,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("rates/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("static client config is valid");
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self { http, base_url }
    }

    /// The rate on `date`, or the latest one.
    pub async fn rate(&self, base: &str, quote: &str, date: Option<NaiveDate>) -> Result<Rate, Error> {
        let url = format!("{}/rate/{base}/{quote}", self.base_url);
        let query: Vec<(&str, String)> = date.map(|d| ("date", d.to_string())).into_iter().collect();
        self.get(&url, &query).await
    }

    /// Daily rates from `from` to `to`, inclusive. An empty answer is an error.
    pub async fn rates(&self, base: &str, quote: &str, from: NaiveDate, to: NaiveDate) -> Result<Vec<Rate>, Error> {
        let url = format!("{}/rates", self.base_url);
        let query = [
            ("from", from.to_string()),
            ("to", to.to_string()),
            ("base", base.to_string()),
            ("quotes", quote.to_string()),
        ];
        let rates: Vec<Rate> = self.get(&url, &query).await?;
        if rates.is_empty() {
            return Err(Error::EmptyRange);
        }
        Ok(rates)
    }

    async fn get<T: DeserializeOwned>(&self, url: &str, query: &[(&str, String)]) -> Result<T, Error> {
        let failed = |reason: String| Error::Request {
            url: url.to_string(),
            reason,
        };

        let response = self
            .http
            .get(url)
            .query(query)
            .send()
            .await
            .map_err(|e| failed(e.to_string()))?;
        let status = response.status();
        let body = response.text().await.map_err(|e| failed(e.to_string()))?;

        if !status.is_success() {
            let message = serde_json::from_str::<ApiError>(&body)
                .map(|e| e.message)
                .unwrap_or(body);
            return Err(Error::Api {
                status: status.as_u16(),
                message,
            });
        }
        serde_json::from_str(&body).map_err(|e| failed(format!("unexpected response: {e}")))
    }
}
