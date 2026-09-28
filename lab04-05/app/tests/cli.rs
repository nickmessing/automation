mod common;

use common::*;
use rates::Error;
use rates::cli::{self, Output, RateArgs};
use rates::store::Store;

fn args(base: &str, quote: &str) -> RateArgs {
    RateArgs {
        base: base.into(),
        quote: quote.into(),
        ..Default::default()
    }
}

async fn run(args: &RateArgs, output: Output) -> (Result<(), Error>, String, tempfile::TempDir) {
    let (_server, client) = frankfurter().await;
    let dir = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    let result = cli::run(&client, &Store::new(dir.path()), args, today(), output, &mut out).await;
    (result, String::from_utf8(out).unwrap(), dir)
}

#[tokio::test]
async fn single_date_prints_and_saves() {
    let args = RateArgs {
        date: Some("2024-03-15".into()),
        ..args("eur", "usd")
    };
    let (result, out, dir) = run(&args, Output::default()).await;

    result.unwrap();
    let file = dir.path().join("data/EUR-USD-2024-03-15.json");
    assert_eq!(
        out,
        format!("1 EUR = 1.0902 USD on 2024-03-15\nsaved to {}\n", file.display())
    );
    assert!(file.exists());
}

#[tokio::test]
async fn range_prints_stats_and_saves_one_file() {
    let args = RateArgs {
        from: Some("2024-03-01".into()),
        to: Some("2024-03-03".into()),
        ..args("EUR", "USD")
    };
    let (result, out, dir) = run(&args, Output::default()).await;

    result.unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "EUR → USD, 2024-03-01 … 2024-03-03, 3 days");
    assert_eq!(
        lines[1],
        "low 1.0821 (2024-03-02)  high 1.0823 (2024-03-01)  change -0.01%"
    );
    assert!(dir.path().join("data/EUR-USD-2024-03-01_2024-03-03.json").exists());
}

#[tokio::test]
async fn range_draws_a_graph_when_asked() {
    let args = RateArgs {
        from: Some("2024-03-01".into()),
        to: Some("2024-03-03".into()),
        ..args("EUR", "USD")
    };
    let (result, out, _dir) = run(
        &args,
        Output {
            graph: Some((60, 20)),
            color: false,
        },
    )
    .await;

    result.unwrap();
    assert!(out.contains('┤') && out.contains('└'), "{out}");
    // header + 12 chart rows + axis + dates + stats + saved
    assert_eq!(out.lines().count(), 1 + 12 + 2 + 2);
}

#[tokio::test]
async fn input_errors_are_caught_before_the_request() {
    let cases = [
        (args("EU", "USD"), Error::InvalidCurrency("EU".into())),
        (
            RateArgs {
                date: Some("garbage".into()),
                ..args("EUR", "USD")
            },
            Error::InvalidDate("garbage".into()),
        ),
        (
            RateArgs {
                date: Some("2030-01-01".into()),
                ..args("EUR", "USD")
            },
            Error::FutureDate(day("2030-01-01")),
        ),
        (
            RateArgs {
                from: Some("2024-03-10".into()),
                to: Some("2024-03-01".into()),
                ..args("EUR", "USD")
            },
            Error::RangeOrder {
                from: day("2024-03-10"),
                to: day("2024-03-01"),
            },
        ),
        (
            RateArgs {
                date: Some("2024-03-01".into()),
                from: Some("2024-03-01".into()),
                ..args("EUR", "USD")
            },
            Error::Usage("pass either a date or --from/--to, not both"),
        ),
        (
            RateArgs {
                to: Some("2024-03-01".into()),
                ..args("EUR", "USD")
            },
            Error::Usage("--to needs a --from (add --from YYYY-MM-DD)"),
        ),
    ];
    for (args, expected) in cases {
        let (result, out, dir) = run(&args, Output::default()).await;
        assert_eq!(result, Err(expected));
        assert_eq!(out, "");
        assert!(!dir.path().join("data").exists());
    }
}

#[tokio::test]
async fn api_errors_are_returned() {
    let (result, _, _) = run(&args("EUR", "XXX"), Output::default()).await;
    assert_eq!(
        result,
        Err(Error::Api {
            status: 422,
            message: "invalid currency: XXX".into()
        })
    );

    let args = RateArgs {
        from: Some("1950-01-01".into()),
        to: Some("1950-02-01".into()),
        ..args("EUR", "USD")
    };
    let (result, _, _) = run(&args, Output::default()).await;
    assert_eq!(result, Err(Error::EmptyRange));
}

#[test]
fn error_log_context_matches_lab02() {
    let today = today();
    assert_eq!(args("EUR", "XXX").context(today), "EUR->XXX latest");
    assert_eq!(
        RateArgs {
            date: Some("2024-03-15".into()),
            ..args("EUR", "USD")
        }
        .context(today),
        "EUR->USD 2024-03-15"
    );
    assert_eq!(
        RateArgs {
            from: Some("2024-03-01".into()),
            ..args("EUR", "USD")
        }
        .context(today),
        "EUR->USD 2024-03-01..2026-09-28"
    );
    assert_eq!(
        RateArgs {
            to: Some("2024-03-01".into()),
            ..args("EUR", "USD")
        }
        .context(today),
        "EUR->USD ?..2024-03-01"
    );
}
