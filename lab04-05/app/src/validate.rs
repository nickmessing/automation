//! Input checks done before calling the API, so obvious mistakes get a clear message.

use chrono::NaiveDate;

use crate::Error;

/// A 3-letter currency code, upper-cased.
pub fn currency(code: &str) -> Result<String, Error> {
    let upper = code.to_ascii_uppercase();
    if upper.len() == 3 && upper.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(upper)
    } else {
        Err(Error::InvalidCurrency(code.to_string()))
    }
}

/// A `YYYY-MM-DD` date that is not after `today`.
pub fn date(value: &str, today: NaiveDate) -> Result<NaiveDate, Error> {
    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| Error::InvalidDate(value.to_string()))?;
    if parsed > today {
        return Err(Error::FutureDate(parsed));
    }
    Ok(parsed)
}

/// A range whose start is not after its end.
pub fn range(from: NaiveDate, to: NaiveDate) -> Result<(), Error> {
    if from > to {
        return Err(Error::RangeOrder { from, to });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn currency_is_upper_cased() {
        assert_eq!(currency("mdl").unwrap(), "MDL");
        assert_eq!(currency("Eur").unwrap(), "EUR");
    }

    #[test]
    fn currency_must_be_three_letters() {
        for bad in ["EU", "EURO", "E1R", "", "€UR"] {
            assert_eq!(currency(bad), Err(Error::InvalidCurrency(bad.into())), "{bad}");
        }
    }

    #[test]
    fn date_parses_iso_format() {
        assert_eq!(date("2024-03-15", day("2026-01-01")).unwrap(), day("2024-03-15"));
    }

    #[test]
    fn date_rejects_garbage_and_impossible_dates() {
        let today = day("2026-01-01");
        for bad in ["garbage", "2024-02-30", "15.03.2024", ""] {
            assert_eq!(date(bad, today), Err(Error::InvalidDate(bad.into())), "{bad}");
        }
    }

    #[test]
    fn date_rejects_the_future_but_allows_today() {
        let today = day("2026-01-01");
        assert!(date("2026-01-01", today).is_ok());
        assert_eq!(date("2026-01-02", today), Err(Error::FutureDate(day("2026-01-02"))));
    }

    #[test]
    fn range_must_be_in_order() {
        assert!(range(day("2024-01-01"), day("2024-01-01")).is_ok());
        assert!(matches!(
            range(day("2024-02-01"), day("2024-01-01")),
            Err(Error::RangeOrder { .. })
        ));
    }
}
