//! The `rates rate` command: lab02's behaviour, one date or a range.

use std::io::Write;

use chrono::NaiveDate;

use crate::stats::Stats;
use crate::store::Store;
use crate::{Client, Error, chart, validate};

#[derive(Debug, Clone, Default)]
pub struct RateArgs {
    pub base: String,
    pub quote: String,
    pub date: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl RateArgs {
    fn is_range(&self) -> bool {
        self.from.is_some() || self.to.is_some()
    }

    /// How the request is described in `error.log`, e.g. `EUR->USD latest`.
    pub fn context(&self, today: NaiveDate) -> String {
        if self.is_range() {
            let from = self.from.as_deref().unwrap_or("?");
            let to = self.to.clone().unwrap_or_else(|| today.to_string());
            format!("{}->{} {from}..{to}", self.base, self.quote)
        } else {
            format!(
                "{}->{} {}",
                self.base,
                self.quote,
                self.date.as_deref().unwrap_or("latest")
            )
        }
    }
}

/// How to print: `graph` is the terminal size to draw into, `None` skips the graph.
#[derive(Debug, Clone, Copy, Default)]
pub struct Output {
    pub graph: Option<(usize, usize)>,
    pub color: bool,
}

pub async fn run(
    client: &Client,
    store: &Store,
    args: &RateArgs,
    today: NaiveDate,
    output: Output,
    out: &mut impl Write,
) -> Result<(), Error> {
    if args.is_range() {
        range(client, store, args, today, output, out).await
    } else {
        single(client, store, args, today, out).await
    }
}

async fn single(
    client: &Client,
    store: &Store,
    args: &RateArgs,
    today: NaiveDate,
    out: &mut impl Write,
) -> Result<(), Error> {
    let base = validate::currency(&args.base)?;
    let quote = validate::currency(&args.quote)?;
    let date = args.date.as_deref().map(|d| validate::date(d, today)).transpose()?;

    let rate = client.rate(&base, &quote, date).await?;
    let file = store.save_rate(&rate).map_err(|e| Error::Save(e.to_string()))?;

    let _ = writeln!(out, "1 {} = {} {} on {}", rate.base, rate.rate, rate.quote, rate.date);
    let _ = writeln!(out, "saved to {}", file.display());
    Ok(())
}

async fn range(
    client: &Client,
    store: &Store,
    args: &RateArgs,
    today: NaiveDate,
    output: Output,
    out: &mut impl Write,
) -> Result<(), Error> {
    if args.date.is_some() {
        return Err(Error::Usage("pass either a date or --from/--to, not both"));
    }
    let Some(from) = args.from.as_deref() else {
        return Err(Error::Usage("--to needs a --from (add --from YYYY-MM-DD)"));
    };
    let base = validate::currency(&args.base)?;
    let quote = validate::currency(&args.quote)?;
    let from = validate::date(from, today)?;
    let to = match args.to.as_deref() {
        Some(to) => validate::date(to, today)?,
        None => today,
    };
    validate::range(from, to)?;

    let rates = client.rates(&base, &quote, from, to).await?;
    let file = store.save_range(&rates).map_err(|e| Error::Save(e.to_string()))?;
    let stats = Stats::of(&rates).expect("the client never returns an empty range");

    let (bold, reset) = if output.color {
        ("\x1b[1;37m", "\x1b[0m")
    } else {
        ("", "")
    };
    let days = if rates.len() == 1 { "day" } else { "days" };
    let _ = writeln!(
        out,
        "{bold}{base} → {quote}{reset}, {} … {}, {} {days}",
        stats.first.date,
        stats.last.date,
        rates.len()
    );
    if let Some((cols, rows)) = output.graph {
        let _ = write!(out, "{}", chart::render(&rates, cols, rows, output.color));
    }
    let change_color = match (output.color, stats.change >= 0.0) {
        (false, _) => "",
        (true, true) => "\x1b[32m",
        (true, false) => "\x1b[31m",
    };
    let _ = writeln!(
        out,
        "low {} ({})  high {} ({})  change {change_color}{:+.2}%{reset}",
        stats.low.rate, stats.low.date, stats.high.rate, stats.high.date, stats.percent
    );
    let _ = writeln!(out, "saved to {}", file.display());
    Ok(())
}
