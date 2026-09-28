//! `rates serve`: a chart page and a small JSON API on top of the Frankfurter client.

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{Days, Local, NaiveDate};
use serde::Deserialize;
use serde_json::json;

use crate::stats::Stats;
use crate::svg::{self, escape};
use crate::{Client, Error, Rate, validate};

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    /// Injected so tests do not depend on the current date.
    pub today: fn() -> NaiveDate,
}

impl AppState {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            today: || Local::now().date_naive(),
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/api/rate/{base}/{quote}", get(api_rate))
        .route("/api/rates/{base}/{quote}", get(api_rates))
        .layer(middleware::from_fn(log_requests))
        .with_state(state)
}

/// Serve until SIGINT or SIGTERM (what systemd sends on stop).
pub async fn serve(addr: SocketAddr, state: AppState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown())
        .await
}

async fn shutdown() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("can listen for SIGTERM");
        tokio::select! {
            _ = interrupt => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = interrupt.await;
    println!("shutting down");
}

async fn log_requests(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let started = Instant::now();
    let response = next.run(request).await;
    println!(
        "{method} {uri} {} {}ms",
        response.status().as_u16(),
        started.elapsed().as_millis()
    );
    response
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// Errors as `{"status", "message", "help"}` with a matching HTTP status.
struct ApiError(Error);

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        ApiError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = json!({ "status": status.as_u16(), "message": self.0.to_string(), "help": self.0.help() });
        (status, Json(body)).into_response()
    }
}

#[derive(Deserialize)]
struct DateQuery {
    date: Option<String>,
}

async fn api_rate(
    State(state): State<AppState>,
    Path((base, quote)): Path<(String, String)>,
    Query(query): Query<DateQuery>,
) -> Result<Json<Rate>, ApiError> {
    let today = (state.today)();
    let base = validate::currency(&base)?;
    let quote = validate::currency(&quote)?;
    let date = query.date.as_deref().map(|d| validate::date(d, today)).transpose()?;
    Ok(Json(state.client.rate(&base, &quote, date).await?))
}

#[derive(Deserialize)]
struct RangeQuery {
    from: Option<String>,
    to: Option<String>,
}

async fn api_rates(
    State(state): State<AppState>,
    Path((base, quote)): Path<(String, String)>,
    Query(query): Query<RangeQuery>,
) -> Result<Json<Vec<Rate>>, ApiError> {
    let today = (state.today)();
    let base = validate::currency(&base)?;
    let quote = validate::currency(&quote)?;
    let from = validate::date(query.from.as_deref().ok_or(Error::MissingParam("from"))?, today)?;
    let to = match query.to.as_deref() {
        Some(to) => validate::date(to, today)?,
        None => today,
    };
    validate::range(from, to)?;
    Ok(Json(state.client.rates(&base, &quote, from, to).await?))
}

#[derive(Deserialize, Default)]
struct Form {
    base: Option<String>,
    quote: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

/// The form's values, with defaults for anything left empty.
struct Filled {
    base: String,
    quote: String,
    from: String,
    to: String,
}

impl Form {
    fn fill(self, today: NaiveDate) -> Filled {
        let or = |v: Option<String>, default: String| v.filter(|v| !v.trim().is_empty()).unwrap_or(default);
        Filled {
            base: or(self.base, "EUR".into()),
            quote: or(self.quote, "USD".into()),
            from: or(self.from, (today - Days::new(29)).to_string()),
            to: or(self.to, today.to_string()),
        }
    }
}

async fn index(State(state): State<AppState>, Query(form): Query<Form>) -> (StatusCode, Html<String>) {
    let today = (state.today)();
    let form = form.fill(today);

    let result = async {
        let base = validate::currency(&form.base)?;
        let quote = validate::currency(&form.quote)?;
        let from = validate::date(&form.from, today)?;
        let to = validate::date(&form.to, today)?;
        validate::range(from, to)?;
        state.client.rates(&base, &quote, from, to).await
    }
    .await;

    let (status, body) = match result {
        Ok(rates) => (StatusCode::OK, chart_section(&rates)),
        Err(e) => (
            StatusCode::from_u16(e.status()).unwrap_or(StatusCode::BAD_REQUEST),
            error_section(&e),
        ),
    };
    (status, Html(page(&form, today, &body)))
}

fn chart_section(rates: &[Rate]) -> String {
    let stats = Stats::of(rates).expect("the client never returns an empty range");
    let trend = if stats.change >= 0.0 { "up" } else { "down" };
    let api = format!(
        "/api/rates/{}/{}?from={}&amp;to={}",
        stats.first.base, stats.first.quote, stats.first.date, stats.last.date
    );
    format!(
        r#"<section class="card">
  <h2>{base} → {quote} <span class="muted">{from} … {to} · {n} days</span></h2>
  {chart}
  <dl class="stats">
    <div><dt>Latest</dt><dd>{last_rate}</dd></div>
    <div><dt>Low</dt><dd>{low} <span class="muted">{low_date}</span></dd></div>
    <div><dt>High</dt><dd>{high} <span class="muted">{high_date}</span></dd></div>
    <div><dt>Change</dt><dd class="{trend}">{percent:+.2}%</dd></div>
  </dl>
  <p class="muted"><a href="{api}">This range as JSON</a></p>
</section>"#,
        base = escape(&stats.first.base),
        quote = escape(&stats.first.quote),
        from = stats.first.date,
        to = stats.last.date,
        n = rates.len(),
        chart = svg::render(rates),
        last_rate = stats.last.rate,
        low = stats.low.rate,
        low_date = stats.low.date,
        high = stats.high.rate,
        high_date = stats.high.date,
        percent = stats.percent,
    )
}

fn error_section(error: &Error) -> String {
    let help = error
        .help()
        .map(|h| format!("<p>{}</p>", escape(h)))
        .unwrap_or_default();
    format!(
        r#"<section class="card error" role="alert"><h2>{}</h2>{help}</section>"#,
        escape(&error.to_string())
    )
}

fn page(form: &Filled, today: NaiveDate, body: &str) -> String {
    let presets: String = [("30 days", 29), ("90 days", 89), ("1 year", 364), ("5 years", 1825)]
        .iter()
        .map(|(label, days)| {
            let from = today - Days::new(*days);
            format!(
                r#"<a href="/?base={}&amp;quote={}&amp;from={from}&amp;to={today}">{label}</a>"#,
                escape(&form.base),
                escape(&form.quote)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Exchange rates</title>
<style>
:root {{ --bg: #f7f7f5; --card: #fff; --text: #1c1c1a; --muted: #6b6b66; --line: #2563eb; --area: rgba(37, 99, 235, .12); --grid: #e4e4e0; --up: #15803d; --down: #b91c1c; --error-bg: #fef2f2; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg: #141413; --card: #1f1f1d; --text: #ececea; --muted: #9a9a94; --line: #60a5fa; --area: rgba(96, 165, 250, .15); --grid: #33332f; --up: #4ade80; --down: #f87171; --error-bg: #3b1d1d; }} }}
* {{ box-sizing: border-box; }}
body {{ margin: 0; background: var(--bg); color: var(--text); font: 16px/1.5 system-ui, sans-serif; }}
main {{ max-width: 880px; margin: 0 auto; padding: 24px 16px; }}
h1 {{ font-size: 1.5rem; margin: 0 0 16px; }}
h2 {{ font-size: 1.1rem; margin: 0 0 12px; }}
a {{ color: var(--line); }}
.card {{ background: var(--card); border-radius: 12px; padding: 16px; margin-bottom: 16px; box-shadow: 0 1px 2px rgba(0, 0, 0, .06); }}
.error {{ background: var(--error-bg); }}
.error p {{ margin: 0; color: var(--muted); }}
form {{ display: flex; flex-wrap: wrap; gap: 12px; align-items: end; }}
label {{ display: flex; flex-direction: column; font-size: .85rem; color: var(--muted); }}
input {{ font: inherit; padding: 6px 8px; border: 1px solid var(--grid); border-radius: 6px; background: var(--bg); color: var(--text); }}
input[name=base], input[name=quote] {{ width: 5em; text-transform: uppercase; }}
button {{ font: inherit; padding: 7px 16px; border: 0; border-radius: 6px; background: var(--line); color: #fff; cursor: pointer; }}
.presets {{ margin: 12px 0 0; font-size: .9rem; }}
.muted {{ color: var(--muted); font-weight: normal; }}
.chart {{ width: 100%; height: auto; display: block; }}
.chart .grid, .chart .axis {{ stroke: var(--grid); }}
.chart .label {{ fill: var(--muted); font-size: 12px; }}
.chart .line {{ fill: none; stroke: var(--line); stroke-width: 2; stroke-linejoin: round; }}
.chart .area {{ fill: var(--area); }}
.chart .dot {{ fill: var(--line); }}
.stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; margin: 16px 0 8px; }}
.stats dt {{ font-size: .85rem; color: var(--muted); }}
.stats dd {{ margin: 0; font-size: 1.2rem; font-variant-numeric: tabular-nums; }}
.up {{ color: var(--up); }} .down {{ color: var(--down); }}
footer {{ font-size: .85rem; color: var(--muted); }}
</style>
</head>
<body>
<main>
<h1>Exchange rates</h1>
<section class="card">
<form method="get" action="/">
  <label>From currency <input name="base" value="{base}" maxlength="3" required></label>
  <label>To currency <input name="quote" value="{quote}" maxlength="3" required></label>
  <label>Start <input type="date" name="from" value="{from}" max="{today}"></label>
  <label>End <input type="date" name="to" value="{to}" max="{today}"></label>
  <button type="submit">Show</button>
</form>
<p class="presets muted">Last {presets}</p>
</section>
{body}
<footer>Data from <a href="https://frankfurter.dev">Frankfurter</a> · rates {version} · <a href="/health">health</a></footer>
</main>
</body>
</html>
"#,
        base = escape(&form.base),
        quote = escape(&form.quote),
        from = escape(&form.from),
        to = escape(&form.to),
        version = env!("CARGO_PKG_VERSION"),
    )
}
