//! Braille line chart for the terminal, a port of lab02's `draw-graph`.
//!
//! Each character cell holds 2x4 braille dots, so a `cols x rows` chart has
//! `2*cols x 4*rows` dots of resolution.

use crate::Rate;

// braille dot bits for one cell column, rows top to bottom
const LEFT_DOTS: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
const RIGHT_DOTS: [u32; 4] = [0x08, 0x10, 0x20, 0x80];

/// Rows the chart leaves free for the header, x axis, dates, stats and prompt.
const RESERVED_ROWS: usize = 8;

/// A dot column's vertical extent, in dot rows (0 = top).
#[derive(Clone, Copy)]
struct Span {
    top: usize,
    bottom: usize,
}

fn dot_bits(span: Span, row: usize, weights: &[u32; 4]) -> u32 {
    if span.bottom < row || span.top > row + 3 {
        return 0;
    }
    (0..4)
        .filter(|k| (span.top..=span.bottom).contains(&(row + k)))
        .map(|k| weights[k])
        .sum()
}

/// Decimals in the shortest representation of `v`: 1.0902 -> 4, 1.0 -> 0.
fn decimals_of(v: f64) -> usize {
    let s = v.to_string();
    s.split_once('.').map_or(0, |(_, d)| d.len())
}

/// Render `rates` to fit a terminal of `term_cols x term_rows`.
pub fn render(rates: &[Rate], term_cols: usize, term_rows: usize, color: bool) -> String {
    if rates.is_empty() {
        return String::new();
    }
    let (dim, line_color, reset) = if color {
        ("\x1b[90m", "\x1b[36m", "\x1b[0m")
    } else {
        ("", "", "")
    };

    let values: Vec<f64> = rates.iter().map(|r| r.rate).collect();
    let n = values.len();
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // a flat line would divide by zero, give it some room
    let pad = if max == min { max.abs() * 0.01 + 0.0001 } else { 0.0 };
    let (lo, hi) = (min - pad, max + pad);
    let range = hi - lo;

    // enough decimals to tell rows apart, but never more than the data has
    let data_decimals = values.iter().map(|&v| decimals_of(v)).max().unwrap_or(0);
    let wanted = (-range.log10()).ceil() as i64 + 2;
    let decimals = wanted.min(data_decimals as i64).max(2) as usize;
    let fmt = |v: f64| format!("{v:.decimals$}");
    let label_width = fmt(hi).len().max(fmt(lo).len());

    let rows = term_rows.saturating_sub(RESERVED_ROWS).max(5);
    let cols = term_cols.saturating_sub(label_width + 2).max(10);
    let dot_cols = cols * 2;
    let dot_rows = rows * 4;

    // sample the series once per dot column, as a dot row (0 = top)
    let ys: Vec<usize> = (0..dot_cols)
        .map(|x| {
            let t = if n == 1 {
                0.0
            } else {
                x as f64 * (n - 1) as f64 / (dot_cols - 1) as f64
            };
            let i = t.floor() as usize;
            let j = (i + 1).min(n - 1);
            let f = t - i as f64;
            let v = values[i] * (1.0 - f) + values[j] * f;
            ((hi - v) / range * (dot_rows - 1) as f64).round() as usize
        })
        .collect();
    // join each sample to the previous one so the line has no gaps
    let spans: Vec<Span> = ys
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            let prev = if i == 0 { y } else { ys[i - 1] };
            Span {
                top: prev.min(y),
                bottom: prev.max(y),
            }
        })
        .collect();

    // a tick every 4 rows from the bottom, plus the top row if there is room,
    // dropping ticks that round to the same label as the one above
    let mut shown: Option<String> = None;
    let ticks: Vec<Option<String>> = (0..rows)
        .map(|r| {
            let from_bottom = (rows - 1 - r) % 4;
            if !(from_bottom == 0 || (r == 0 && from_bottom >= 2)) {
                return None;
            }
            let label = fmt(hi - (r as f64 * 4.0 + 1.5) / (dot_rows - 1) as f64 * range);
            if shown.as_ref() == Some(&label) {
                return None;
            }
            shown = Some(label.clone());
            Some(label)
        })
        .collect();

    let mut out = String::new();
    for (r, tick) in ticks.iter().enumerate() {
        let row = r * 4;
        let line: String = (0..cols)
            .map(|c| {
                let mask = dot_bits(spans[c * 2], row, &LEFT_DOTS) | dot_bits(spans[c * 2 + 1], row, &RIGHT_DOTS);
                if mask == 0 {
                    ' '
                } else {
                    char::from_u32(0x2800 + mask).expect("braille block")
                }
            })
            .collect();
        let axis = match tick {
            Some(label) => format!("{label:>label_width$} ┤"),
            None => format!("{:label_width$} │", ""),
        };
        out.push_str(&format!("{dim}{axis}{reset}{line_color}{line}{reset}\n"));
    }

    // x axis with a date under each tick
    let k = if n == 1 { 1 } else { n.min(((cols - 1) / 20 + 1).max(2)) };
    let x_ticks: Vec<usize> = (0..k)
        .map(|i| {
            if k == 1 {
                0
            } else {
                (i as f64 * (cols - 1) as f64 / (k - 1) as f64).round() as usize
            }
        })
        .collect();
    let axis: String = (0..cols)
        .map(|c| if x_ticks.contains(&c) { '┬' } else { '─' })
        .collect();
    let mut labels = String::new();
    for (i, &tick) in x_ticks.iter().enumerate() {
        let idx = (tick as f64 * (n - 1) as f64 / (cols - 1) as f64).round() as usize;
        let label = rates[idx].date.to_string();
        let start = if i == k - 1 && k > 1 { cols - label.len() } else { tick };
        if labels.len() > start {
            continue;
        }
        labels.push_str(&" ".repeat(start - labels.len()));
        labels.push_str(&label);
    }
    let gutter = " ".repeat(label_width + 1);
    out.push_str(&format!("{dim}{gutter}└{axis}{reset}\n"));
    out.push_str(&format!("{gutter} {labels}\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(values: &[f64]) -> Vec<Rate> {
        let start = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        values
            .iter()
            .enumerate()
            .map(|(i, &rate)| Rate {
                date: start + chrono::Days::new(i as u64),
                base: "EUR".into(),
                quote: "USD".into(),
                rate,
            })
            .collect()
    }

    fn width(line: &str) -> usize {
        line.chars().count()
    }

    #[test]
    fn empty_series_renders_nothing() {
        assert_eq!(render(&[], 80, 24, false), "");
    }

    #[test]
    fn fills_the_terminal() {
        let chart = render(&series(&[1.0, 1.5, 1.2, 1.8, 1.1]), 80, 24, false);
        let lines: Vec<&str> = chart.lines().collect();
        // 24 rows - 8 reserved = 16 chart rows, then the axis and the dates
        assert_eq!(lines.len(), 16 + 2);
        for line in &lines[..17] {
            assert_eq!(width(line), 80, "{line:?}");
        }
    }

    #[test]
    fn has_a_minimum_size() {
        let chart = render(&series(&[1.0, 2.0]), 1, 1, false);
        assert_eq!(chart.lines().count(), 5 + 2);
    }

    #[test]
    fn labels_the_y_axis_every_four_rows_from_the_bottom() {
        let chart = render(&series(&[1.0, 2.0]), 80, 24, false);
        let ticked: Vec<usize> = chart
            .lines()
            .enumerate()
            .filter(|(_, l)| l.contains('┤'))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(ticked, vec![0, 3, 7, 11, 15]);
    }

    #[test]
    fn y_labels_never_repeat() {
        // tiny range with 4-decimal data: rows round to the same label
        let chart = render(&series(&[1.0823, 1.0821, 1.0822]), 60, 24, false);
        let labels: Vec<&str> = chart
            .lines()
            .filter(|l| l.contains('┤'))
            .map(|l| l.split('┤').next().unwrap().trim())
            .collect();
        let mut unique = labels.clone();
        unique.dedup();
        assert_eq!(labels, unique);
        assert!(labels.iter().all(|l| l.len() == "1.0823".len()), "{labels:?}");
    }

    #[test]
    fn y_range_fits_the_data_not_zero() {
        let chart = render(&series(&[156.03, 174.87]), 80, 24, false);
        let top = chart.lines().next().unwrap();
        let bottom = chart.lines().rfind(|l| l.contains('┤')).unwrap();
        assert!(top.starts_with("174."), "{top}");
        assert!(bottom.starts_with("156."), "{bottom}");
    }

    #[test]
    fn first_and_last_dates_are_on_the_x_axis() {
        let rates = series(&[1.0; 100]);
        let chart = render(&rates, 100, 24, false);
        let dates = chart.lines().last().unwrap();
        assert!(dates.trim_start().starts_with("2024-01-01"), "{dates}");
        assert!(dates.ends_with("2024-04-09"), "{dates}");
    }

    #[test]
    fn a_flat_series_draws_a_line_in_the_middle() {
        let chart = render(&series(&[1.0; 5]), 40, 24, false);
        let drawn: Vec<usize> = chart
            .lines()
            .enumerate()
            .filter(|(_, l)| l.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(drawn.len(), 1);
        assert!((6..=9).contains(&drawn[0]), "{drawn:?}");
    }

    #[test]
    fn a_single_point_still_renders() {
        let chart = render(&series(&[1.0823]), 40, 12, false);
        assert!(chart.contains("2024-01-01"));
        assert!(chart.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)));
    }

    #[test]
    fn color_is_optional() {
        let rates = series(&[1.0, 2.0]);
        assert!(!render(&rates, 40, 12, false).contains('\x1b'));
        assert!(render(&rates, 40, 12, true).contains("\x1b[36m"));
    }

    #[test]
    fn rising_series_goes_from_bottom_left_to_top_right() {
        let chart = render(&series(&[1.0, 2.0]), 40, 13, false);
        let lines: Vec<&str> = chart.lines().collect();
        let plot = |l: &str| l.split(['┤', '│']).nth(1).unwrap().chars().collect::<Vec<_>>();
        let top = plot(lines[0]);
        let bottom = plot(lines[4]);
        assert_ne!(*top.last().unwrap(), ' ');
        assert_eq!(top[0], ' ');
        assert_ne!(bottom[0], ' ');
    }
}
