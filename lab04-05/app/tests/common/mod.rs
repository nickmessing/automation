#![allow(dead_code)]

use rates::{Client, Rate};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub fn day(s: &str) -> chrono::NaiveDate {
    s.parse().unwrap()
}

pub fn today() -> chrono::NaiveDate {
    day("2026-09-28")
}

pub fn rate(date: &str, base: &str, quote: &str, rate: f64) -> Rate {
    Rate {
        date: day(date),
        base: base.into(),
        quote: quote.into(),
        rate,
    }
}

/// A fake Frankfurter with a few known answers.
pub async fn frankfurter() -> (MockServer, Client) {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rate/EUR/USD"))
        .and(query_param("date", "2024-03-15"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rate("2024-03-15", "EUR", "USD", 1.0902)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rate/EUR/USD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rate("2026-09-28", "EUR", "USD", 1.1234)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rate/EUR/XXX"))
        .respond_with(
            ResponseTemplate::new(422).set_body_json(json!({"status": 422, "message": "invalid currency: XXX"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rates"))
        .and(query_param("quotes", "USD"))
        .and(query_param("from", "2024-03-01"))
        .and(query_param("to", "2024-03-03"))
        .respond_with(ResponseTemplate::new(200).set_body_json([
            rate("2024-03-01", "EUR", "USD", 1.0823),
            rate("2024-03-02", "EUR", "USD", 1.0821),
            rate("2024-03-03", "EUR", "USD", 1.0822),
        ]))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rates"))
        .and(query_param("from", "1950-01-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rates"))
        .and(query_param("quotes", "XXX"))
        .respond_with(
            ResponseTemplate::new(422).set_body_json(json!({"status": 422, "message": "invalid currency: XXX"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rates"))
        .and(query_param("quotes", "JPY"))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream down"))
        .mount(&server)
        .await;

    let client = Client::new(server.uri());
    (server, client)
}
