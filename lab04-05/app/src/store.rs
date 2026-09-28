//! Saving results to `data/` and errors to `error.log`, like lab02.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::Serialize;

use crate::{Error, Rate};

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `data/<BASE>-<QUOTE>-<DATE>.json`
    pub fn save_rate(&self, rate: &Rate) -> io::Result<PathBuf> {
        self.save(&format!("{}-{}-{}", rate.base, rate.quote, rate.date), rate)
    }

    /// `data/<BASE>-<QUOTE>-<FIRST>_<LAST>.json`, named after the dates actually returned.
    pub fn save_range(&self, rates: &[Rate]) -> io::Result<PathBuf> {
        let (Some(first), Some(last)) = (rates.first(), rates.last()) else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "no rates to save"));
        };
        self.save(
            &format!("{}-{}-{}_{}", first.base, first.quote, first.date, last.date),
            rates,
        )
    }

    fn save<T: Serialize + ?Sized>(&self, name: &str, data: &T) -> io::Result<PathBuf> {
        let dir = self.root.join("data");
        fs::create_dir_all(&dir)?;
        let file = dir.join(format!("{name}.json"));
        fs::write(&file, serde_json::to_string_pretty(data)? + "\n")?;
        Ok(file)
    }

    /// Append `<timestamp> [ERROR] <context>: <message>` to `error.log`.
    pub fn log_error(&self, context: &str, error: &Error) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S%:z");
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("error.log"))?;
        writeln!(log, "{now} [ERROR] {context}: {error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(date: &str, value: f64) -> Rate {
        Rate {
            date: date.parse().unwrap(),
            base: "EUR".into(),
            quote: "USD".into(),
            rate: value,
        }
    }

    #[test]
    fn saves_a_single_rate_named_after_its_date() {
        let dir = tempfile::tempdir().unwrap();
        let file = Store::new(dir.path()).save_rate(&rate("2024-03-15", 1.0902)).unwrap();

        assert_eq!(file, dir.path().join("data/EUR-USD-2024-03-15.json"));
        let saved: Rate = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        assert_eq!(saved, rate("2024-03-15", 1.0902));
    }

    #[test]
    fn saves_a_range_named_after_first_and_last_date() {
        let dir = tempfile::tempdir().unwrap();
        let rates = [
            rate("2024-03-01", 1.0),
            rate("2024-03-02", 1.1),
            rate("2024-03-03", 1.2),
        ];
        let file = Store::new(dir.path()).save_range(&rates).unwrap();

        assert_eq!(file, dir.path().join("data/EUR-USD-2024-03-01_2024-03-03.json"));
        let saved: Vec<Rate> = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        assert_eq!(saved, rates);
    }

    #[test]
    fn refuses_to_save_an_empty_range() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Store::new(dir.path()).save_range(&[]).is_err());
    }

    #[test]
    fn appends_errors_to_the_log() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store
            .log_error("EUR->XXX latest", &Error::InvalidCurrency("XXX".into()))
            .unwrap();
        store
            .log_error("EUR->USD garbage", &Error::InvalidDate("garbage".into()))
            .unwrap();

        let log = fs::read_to_string(dir.path().join("error.log")).unwrap();
        let lines: Vec<_> = log.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("[ERROR] EUR->XXX latest: 'XXX' is not a currency code"));
        assert!(lines[1].ends_with("[ERROR] EUR->USD garbage: 'garbage' is not a valid date"));
    }
}
