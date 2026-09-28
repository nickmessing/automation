mod common;

use common::*;
use rates::{Client, Error};

#[tokio::test]
async fn fetches_a_rate_on_a_date() {
    let (_server, client) = frankfurter().await;
    let got = client.rate("EUR", "USD", Some(day("2024-03-15"))).await.unwrap();
    assert_eq!(got, rate("2024-03-15", "EUR", "USD", 1.0902));
}

#[tokio::test]
async fn fetches_the_latest_rate_without_a_date() {
    let (_server, client) = frankfurter().await;
    let got = client.rate("EUR", "USD", None).await.unwrap();
    assert_eq!(got.rate, 1.1234);
}

#[tokio::test]
async fn fetches_a_range() {
    let (_server, client) = frankfurter().await;
    let got = client
        .rates("EUR", "USD", day("2024-03-01"), day("2024-03-03"))
        .await
        .unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(got[2].rate, 1.0822);
}

#[tokio::test]
async fn api_errors_keep_the_message_and_status() {
    let (_server, client) = frankfurter().await;
    let err = client.rate("EUR", "XXX", None).await.unwrap_err();
    assert_eq!(
        err,
        Error::Api {
            status: 422,
            message: "invalid currency: XXX".into()
        }
    );
    assert_eq!(err.to_string(), "API error: invalid currency: XXX (HTTP 422)");
}

#[tokio::test]
async fn non_json_errors_use_the_body() {
    let (_server, client) = frankfurter().await;
    let err = client
        .rates("EUR", "JPY", day("2024-03-01"), day("2024-03-03"))
        .await
        .unwrap_err();
    assert_eq!(
        err,
        Error::Api {
            status: 503,
            message: "upstream down".into()
        }
    );
}

#[tokio::test]
async fn an_empty_range_is_an_error() {
    let (_server, client) = frankfurter().await;
    let err = client
        .rates("EUR", "USD", day("1950-01-01"), day("1950-02-01"))
        .await
        .unwrap_err();
    assert_eq!(err, Error::EmptyRange);
}

#[tokio::test]
async fn network_failures_are_request_errors() {
    // nothing listens on port 9 (discard) on localhost
    let client = Client::new("http://127.0.0.1:9");
    let err = client.rate("EUR", "USD", None).await.unwrap_err();
    assert!(matches!(err, Error::Request { .. }), "{err:?}");
    assert_eq!(err.help(), Some("check your internet connection"));
}
