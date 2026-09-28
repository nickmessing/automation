use crate::Rate;

/// Low, high and change over a series, as printed under lab02's graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub first: Rate,
    pub last: Rate,
    /// The first day with the lowest rate.
    pub low: Rate,
    /// The last day with the highest rate.
    pub high: Rate,
    pub change: f64,
    /// Percent change from first to last, rounded to 2 decimals.
    pub percent: f64,
}

impl Stats {
    pub fn of(rates: &[Rate]) -> Option<Stats> {
        let first = rates.first()?.clone();
        let last = rates.last()?.clone();
        let low = rates.iter().min_by(|a, b| a.rate.total_cmp(&b.rate))?.clone();
        let high = rates.iter().max_by(|a, b| a.rate.total_cmp(&b.rate))?.clone();
        let change = last.rate - first.rate;
        let percent = (change / first.rate * 100.0 * 100.0).round() / 100.0;
        Some(Stats {
            first,
            last,
            low,
            high,
            change,
            percent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(values: &[f64]) -> Vec<Rate> {
        values
            .iter()
            .enumerate()
            .map(|(i, &rate)| Rate {
                date: chrono::NaiveDate::from_ymd_opt(2024, 3, 1 + i as u32).unwrap(),
                base: "EUR".into(),
                quote: "USD".into(),
                rate,
            })
            .collect()
    }

    #[test]
    fn finds_low_high_and_change() {
        let stats = Stats::of(&series(&[1.10, 1.05, 1.20, 1.21])).unwrap();
        assert_eq!(stats.low.rate, 1.05);
        assert_eq!(stats.high.rate, 1.21);
        assert!((stats.change - 0.11).abs() < 1e-9);
        assert_eq!(stats.percent, 10.0);
    }

    #[test]
    fn ties_pick_first_low_and_last_high() {
        let stats = Stats::of(&series(&[1.0, 2.0, 1.0, 2.0])).unwrap();
        assert_eq!(stats.low.date.to_string(), "2024-03-01");
        assert_eq!(stats.high.date.to_string(), "2024-03-04");
    }

    #[test]
    fn negative_change() {
        let stats = Stats::of(&series(&[2.0, 1.0])).unwrap();
        assert_eq!(stats.percent, -50.0);
    }

    #[test]
    fn empty_series_has_no_stats() {
        assert_eq!(Stats::of(&[]), None);
    }
}
