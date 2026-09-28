//! Server-side SVG line chart for the web page. Colors come from CSS classes,
//! so the page can theme it for light and dark mode.

use std::fmt::Write;

use crate::Rate;

const WIDTH: f64 = 800.0;
const HEIGHT: f64 = 320.0;
const LEFT: f64 = 64.0;
const RIGHT: f64 = 20.0;
const TOP: f64 = 16.0;
const BOTTOM: f64 = 36.0;

/// Escape text for HTML/SVG content and attribute values.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Round axis ticks (1, 2 or 5 times a power of ten) covering `lo..=hi`.
pub fn nice_ticks(lo: f64, hi: f64, target: usize) -> (Vec<f64>, f64) {
    let raw = (hi - lo) / target.max(1) as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let step = magnitude
        * match raw / magnitude {
            n if n < 1.5 => 1.0,
            n if n < 3.0 => 2.0,
            n if n < 7.0 => 5.0,
            _ => 10.0,
        };
    let start = (lo / step).floor() as i64;
    let end = (hi / step).ceil() as i64;
    ((start..=end).map(|i| i as f64 * step).collect(), step)
}

pub fn render(rates: &[Rate]) -> String {
    let (Some(first), Some(last)) = (rates.first(), rates.last()) else {
        return String::new();
    };

    let min = rates.iter().map(|r| r.rate).fold(f64::INFINITY, f64::min);
    let max = rates.iter().map(|r| r.rate).fold(f64::NEG_INFINITY, f64::max);
    let pad = if max == min { max.abs() * 0.01 + 0.0001 } else { 0.0 };
    let (ticks, step) = nice_ticks(min - pad, max + pad, 5);
    let (lo, hi) = (ticks[0], ticks[ticks.len() - 1]);
    let decimals = (-step.log10().floor()).clamp(0.0, 6.0) as usize;

    let days = (last.date - first.date).num_days() as f64;
    let x = |r: &Rate| {
        let t = if days == 0.0 {
            0.5
        } else {
            (r.date - first.date).num_days() as f64 / days
        };
        LEFT + t * (WIDTH - LEFT - RIGHT)
    };
    let y = |v: f64| TOP + (hi - v) / (hi - lo) * (HEIGHT - TOP - BOTTOM);
    let bottom = HEIGHT - BOTTOM;

    let mut svg = String::new();
    let label = format!(
        "{} to {} exchange rate from {} to {}",
        first.base, first.quote, first.date, last.date
    );
    let _ = write!(
        svg,
        r#"<svg class="chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="{}" xmlns="http://www.w3.org/2000/svg">"#,
        escape(&label)
    );

    for tick in &ticks {
        let ty = y(*tick);
        let _ = write!(
            svg,
            r#"<line class="grid" x1="{LEFT}" x2="{:.1}" y1="{ty:.1}" y2="{ty:.1}"/><text class="label" x="{:.1}" y="{:.1}" text-anchor="end">{tick:.decimals$}</text>"#,
            WIDTH - RIGHT,
            LEFT - 8.0,
            ty + 4.0,
        );
    }

    // up to 6 evenly spaced dates, always including the first and last
    let count = rates.len().min(6);
    let mut shown = Vec::new();
    for i in 0..count {
        let idx = if count == 1 {
            0
        } else {
            i * (rates.len() - 1) / (count - 1)
        };
        if shown.last() == Some(&idx) {
            continue;
        }
        shown.push(idx);
        let r = &rates[idx];
        let anchor = match (i, count) {
            (_, 1) => "middle",
            (0, _) => "start",
            (i, c) if i == c - 1 => "end",
            _ => "middle",
        };
        let _ = write!(
            svg,
            r#"<text class="label" x="{:.1}" y="{:.1}" text-anchor="{anchor}">{}</text>"#,
            x(r),
            bottom + 22.0,
            r.date
        );
    }

    let points: Vec<String> = rates.iter().map(|r| format!("{:.1},{:.1}", x(r), y(r.rate))).collect();
    let _ = write!(
        svg,
        r#"<path class="area" d="M{:.1},{bottom:.1} L{} L{:.1},{bottom:.1} Z"/>"#,
        x(first),
        points.join(" L"),
        x(last)
    );
    let _ = write!(svg, r#"<polyline class="line" points="{}"/>"#, points.join(" "));
    let _ = write!(
        svg,
        r#"<line class="axis" x1="{LEFT}" x2="{:.1}" y1="{bottom:.1}" y2="{bottom:.1}"/>"#,
        WIDTH - RIGHT
    );

    // dots on every point for short ranges, otherwise just the low and high
    let low = rates
        .iter()
        .min_by(|a, b| a.rate.total_cmp(&b.rate))
        .expect("not empty");
    let high = rates
        .iter()
        .max_by(|a, b| a.rate.total_cmp(&b.rate))
        .expect("not empty");
    let marked: Vec<&Rate> = if rates.len() <= 45 {
        rates.iter().collect()
    } else {
        vec![low, high]
    };
    for r in marked {
        let _ = write!(
            svg,
            r#"<circle class="dot" cx="{:.1}" cy="{:.1}" r="3"><title>{}: {}</title></circle>"#,
            x(r),
            y(r.rate),
            r.date,
            r.rate
        );
    }
    svg.push_str("</svg>");
    svg
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

    fn polyline_points(svg: &str) -> Vec<(f64, f64)> {
        let needle = r#"<polyline class="line" points=""#;
        let start = svg.find(needle).unwrap() + needle.len();
        let end = start + svg[start..].find('"').unwrap();
        svg[start..end]
            .split(' ')
            .map(|p| {
                let (x, y) = p.split_once(',').unwrap();
                (x.parse().unwrap(), y.parse().unwrap())
            })
            .collect()
    }

    #[test]
    fn escapes_markup() {
        assert_eq!(escape(r#"<a href="x">&'"#), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;");
    }

    #[test]
    fn nice_ticks_are_round_and_cover_the_range() {
        let (ticks, step) = nice_ticks(1.0399, 1.1174, 5);
        assert_eq!(step, 0.02);
        assert!(ticks[0] <= 1.0399 && *ticks.last().unwrap() >= 1.1174);
        let (ticks, step) = nice_ticks(156.03, 174.87, 5);
        assert_eq!(step, 5.0);
        assert_eq!(ticks, vec![155.0, 160.0, 165.0, 170.0, 175.0]);
    }

    #[test]
    fn empty_series_renders_nothing() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn one_point_per_rate_inside_the_plot() {
        let svg = render(&series(&[1.08, 1.09, 1.07, 1.10]));
        let points = polyline_points(&svg);
        assert_eq!(points.len(), 4);
        for (x, y) in points {
            assert!((LEFT..=WIDTH - RIGHT).contains(&x), "x={x}");
            assert!((TOP..=HEIGHT - BOTTOM).contains(&y), "y={y}");
        }
    }

    #[test]
    fn higher_rates_are_drawn_higher() {
        let points = polyline_points(&render(&series(&[1.0, 2.0])));
        assert!(points[1].1 < points[0].1);
    }

    #[test]
    fn flat_and_single_point_series_have_no_nan() {
        for values in [&[1.0; 5][..], &[1.0823][..]] {
            let svg = render(&series(values));
            assert!(!svg.contains("NaN") && !svg.contains("inf"), "{svg}");
        }
    }

    #[test]
    fn long_series_only_mark_low_and_high() {
        let values: Vec<f64> = (0..100).map(|i| 1.0 + (i as f64 / 10.0).sin() / 10.0).collect();
        let svg = render(&series(&values));
        assert_eq!(svg.matches("<circle").count(), 2);
        assert_eq!(render(&series(&values[..10])).matches("<circle").count(), 10);
    }

    #[test]
    fn labels_first_and_last_date() {
        let svg = render(&series(&[1.0; 30]));
        assert!(svg.contains(">2024-01-01</text>"));
        assert!(svg.contains(">2024-01-30</text>"));
        assert!(svg.contains(r#"aria-label="EUR to USD exchange rate from 2024-01-01 to 2024-01-30""#));
    }
}
