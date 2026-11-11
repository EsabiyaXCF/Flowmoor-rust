// Values and pointer sensitivity follow Flowmoor's TimeRangeInput/timeRangeDrag.
pub fn clock(seconds: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
pub fn compact_clock(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}
pub fn precision(duration: Option<u64>, up: f32) -> u64 {
    let base = match duration {
        None => 300,
        Some(n) if n >= 18000 => 120,
        Some(n) if n >= 7200 => 60,
        Some(n) if n >= 3600 => 30,
        Some(n) if n >= 1800 => 15,
        Some(n) if n >= 600 => 10,
        _ => 5,
    };
    let steps: Vec<_> = [
        300, 240, 180, 120, 90, 60, 45, 30, 20, 15, 10, 8, 6, 5, 4, 3, 2, 1,
    ]
    .into_iter()
    .filter(|s| *s <= base)
    .collect();
    steps[((up / 96.).clamp(0., 1.) * (steps.len() - 1) as f32).round() as usize]
}
pub fn scrub(original: u64, dx: f32, up: f32, duration: Option<u64>) -> u64 {
    let intent = ((up.abs() - 10.) / 86.).clamp(0., 1.);
    let x = if up.abs() <= 10. {
        dx
    } else {
        dx.signum() * (dx.abs() - (4. + 14. * intent)).max(0.) * (1. - 0.82 * intent)
    };
    (original as f64 + (x / 3.).round() as f64 * precision(duration, up) as f64).max(0.) as u64
}
pub fn range(
    start_field: bool,
    next: u64,
    start: u64,
    end: Option<u64>,
    max: Option<u64>,
) -> (u64, Option<u64>) {
    let max = max.unwrap_or(u64::MAX);
    if start_field {
        let a = next.min(max.saturating_sub(1));
        (
            a,
            Some(
                end.filter(|b| *b > a)
                    .unwrap_or_else(|| a.saturating_add(1).min(max)),
            ),
        )
    } else {
        let b = next.max(1).min(max);
        (
            if start >= b {
                b.saturating_sub(1)
            } else {
                start
            },
            Some(b),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_precision_and_crossing_rules() {
        assert_eq!(precision(Some(20000), 0.), 120);
        assert_eq!(precision(Some(19), 96.), 1);
        assert_eq!(scrub(10, 90., 96., Some(19)), 14);
        assert_eq!(scrub(10, 10., 96., Some(19)), 10);
        assert_eq!(scrub(10, 6., 0., Some(19)), 20);
        assert_eq!(range(true, 12, 0, Some(5), Some(19)), (12, Some(13)));
        assert_eq!(range(false, 4, 10, Some(15), Some(19)), (3, Some(4)));
        assert_eq!(range(true, 99, 0, None, Some(19)), (18, Some(19)));
    }
}
