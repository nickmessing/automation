mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
use rates::web::{AppState, router};
use serde_json::Value;
use tower::ServiceExt;

async fn get(uri: &str) -> (StatusCode, String) {
    let (_server, client) = frankfurter().await;
    let app = router(AppState { client, today });
    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let (status, body) = get(uri).await;
    (
        status,
        serde_json::from_str(&body).unwrap_or_else(|_| panic!("not JSON: {body}")),
    )
}

#[tokio::test]
async fn health_does_not_need_the_api() {
    let (status, body) = get_json("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn api_rate_on_a_date() {
    let (status, body) = get_json("/api/rate/eur/usd?date=2024-03-15").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        serde_json::json!({"date": "2024-03-15", "base": "EUR", "quote": "USD", "rate": 1.0902})
    );
}

#[tokio::test]
async fn api_rate_errors_are_json_with_status_and_help() {
    let (status, body) = get_json("/api/rate/EUR/XXX").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["message"], "API error: invalid currency: XXX (HTTP 422)");
    assert_eq!(body["help"], "check the currency codes and date");

    let (status, body) = get_json("/api/rate/EU/USD").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["message"], "'EU' is not a currency code");

    let (status, _) = get_json("/api/rate/EUR/USD?date=2030-01-01").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn api_rates_range() {
    let (status, body) = get_json("/api/rates/EUR/USD?from=2024-03-01&to=2024-03-03").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn api_rates_needs_from() {
    let (status, body) = get_json("/api/rates/EUR/USD").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "missing required parameter 'from'");
}

#[tokio::test]
async fn api_rates_empty_range_is_404() {
    let (status, _) = get_json("/api/rates/EUR/USD?from=1950-01-01&to=1950-02-01").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn index_renders_a_chart_and_stats() {
    let (status, html) = get("/?base=eur&quote=usd&from=2024-03-01&to=2024-03-03").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<svg class=\"chart\""));
    assert!(html.contains("EUR → USD"));
    assert!(html.contains("<dd>1.0822</dd>"), "latest rate shown");
    assert!(html.contains("value=\"eur\""), "form keeps what was typed");
}

#[tokio::test]
async fn index_shows_errors_in_the_page() {
    let (status, html) = get("/?base=EUR&quote=XXX&from=2024-03-01&to=2024-03-03").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("invalid currency: XXX"));
    assert!(!html.contains("<svg"));
}

#[tokio::test]
async fn index_escapes_user_input() {
    let (_, html) = get("/?base=%3Cscript%3E&quote=USD").await;
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
}

#[tokio::test]
async fn index_defaults_to_the_last_30_days() {
    let (_, html) = get("/").await;
    assert!(html.contains("name=\"from\" value=\"2026-08-30\""));
    assert!(html.contains("name=\"to\" value=\"2026-09-28\""));
}
