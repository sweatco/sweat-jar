//! Timestamp and amount parsers for replay inputs.
//!
//! Hand-rolled UTC date math (Howard Hinnant's civil-date algorithm), ported
//! from `contract/src/replay/mod.rs`. No chrono/time dependency.

// Consumed by later replay subcommands (build-db / run); unused until then.
#![allow(dead_code)]

use anyhow::{bail, Context};

/// Snapshot block time (ms): start of the replay window.
pub const H_MS: u64 = 1_774_017_710_156;
/// End of the replay window (ms).
pub const T_END_MS: u64 = 1_788_174_657_961;

/// `"2026-03-20T14:41:54.953Z"` -> `1774017714953`.
///
/// ISO-8601, UTC, optional fractional seconds (padded/truncated to ms), trailing `Z`.
pub fn iso8601_ms_to_epoch_ms(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();
    let s = s
        .strip_suffix('Z')
        .with_context(|| format!("iso8601 timestamp missing trailing 'Z': {s:?}"))?;
    let (date, time) = s
        .split_once('T')
        .with_context(|| format!("iso8601 timestamp missing 'T' separator: {s:?}"))?;

    let (time, frac) = time.split_once('.').map_or((time, None), |(t, f)| (t, Some(f)));
    let ms = match frac {
        None => 0,
        Some(f) => {
            let mut digits: String = f.chars().take(3).collect();
            if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
                bail!("iso8601 fractional seconds not numeric: {f:?}");
            }
            while digits.len() < 3 {
                digits.push('0');
            }
            digits.parse::<u64>().unwrap()
        }
    };

    Ok(datetime_to_epoch_ms(date, time)? + ms)
}

/// `"2026-03-21 19:30:53 UTC"` -> epoch ms. Space-separated, trailing ` UTC`, second precision.
pub fn space_utc_to_epoch_ms(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();
    let s = s
        .strip_suffix(" UTC")
        .with_context(|| format!("timestamp missing trailing ' UTC': {s:?}"))?;
    let (date, time) = s
        .split_once(' ')
        .with_context(|| format!("timestamp missing space separator: {s:?}"))?;
    datetime_to_epoch_ms(date.trim(), time.trim())
}

/// Decimal string of yocto -> `u128`. Trims whitespace. Errors on a decimal point or non-digits.
pub fn yocto_str_to_u128(s: &str) -> anyhow::Result<u128> {
    s.trim()
        .parse::<u128>()
        .map_err(|e| anyhow::anyhow!("invalid yocto amount {s:?}: {e}"))
}

/// `YYYY-MM-DD` + `HH:MM:SS` (UTC) -> epoch ms, zero fraction.
fn datetime_to_epoch_ms(date: &str, time: &str) -> anyhow::Result<u64> {
    let mut d = date.split('-');
    let year = next_field(&mut d, "year", date)?;
    let month = next_field(&mut d, "month", date)?;
    let day = next_field(&mut d, "day", date)?;
    if d.next().is_some() {
        bail!("date has too many fields: {date:?}");
    }

    let mut t = time.split(':');
    let hour = next_field(&mut t, "hour", time)?;
    let minute = next_field(&mut t, "minute", time)?;
    let second = next_field(&mut t, "second", time)?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        bail!("date field out of range: {date:?}");
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=60).contains(&second) {
        bail!("time field out of range: {time:?}");
    }

    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second;
    let secs = u64::try_from(secs).with_context(|| format!("timestamp before epoch: {date:?} {time:?}"))?;
    Ok(secs * 1_000)
}

fn next_field<'a>(it: &mut impl Iterator<Item = &'a str>, name: &str, ctx: &str) -> anyhow::Result<i64> {
    let raw = it.next().with_context(|| format!("missing {name} in {ctx:?}"))?;
    raw.parse::<i64>()
        .with_context(|| format!("non-numeric {name} {raw:?} in {ctx:?}"))
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_ms() {
        assert_eq!(
            iso8601_ms_to_epoch_ms("2026-03-20T14:41:54.953Z").unwrap(),
            1_774_017_714_953
        );
        assert_eq!(
            iso8601_ms_to_epoch_ms("2025-12-19T08:42:06.000Z").unwrap(),
            1_766_133_726_000
        );
    }

    #[test]
    fn iso_no_fraction() {
        assert_eq!(
            iso8601_ms_to_epoch_ms("2026-03-20T14:41:50Z").unwrap(),
            1_774_017_710_000
        );
    }

    #[test]
    fn iso_garbage_errors() {
        assert!(iso8601_ms_to_epoch_ms("garbage").is_err());
    }

    #[test]
    fn space_utc() {
        assert_eq!(
            space_utc_to_epoch_ms("2026-03-21 19:30:53 UTC").unwrap(),
            1_774_121_453_000
        );
        assert_eq!(
            space_utc_to_epoch_ms("2026-08-31 06:35:19 UTC").unwrap(),
            1_788_158_119_000
        );
    }

    #[test]
    fn yocto() {
        assert_eq!(
            yocto_str_to_u128(" 500000000000000000000000 ").unwrap(),
            500_000_000_000_000_000_000_000
        );
        assert!(yocto_str_to_u128("12.5").is_err());
    }

    #[test]
    fn fractional_padding() {
        // ".9" -> 900ms, ".95" -> 950ms, ".953" -> 953ms, beyond 3 digits truncates.
        let base = iso8601_ms_to_epoch_ms("2026-03-20T14:41:50Z").unwrap();
        assert_eq!(iso8601_ms_to_epoch_ms("2026-03-20T14:41:50.9Z").unwrap(), base + 900);
        assert_eq!(iso8601_ms_to_epoch_ms("2026-03-20T14:41:50.95Z").unwrap(), base + 950);
        assert_eq!(iso8601_ms_to_epoch_ms("2026-03-20T14:41:50.953Z").unwrap(), base + 953);
        assert_eq!(
            iso8601_ms_to_epoch_ms("2026-03-20T14:41:50.953999Z").unwrap(),
            base + 953
        );
    }
}
