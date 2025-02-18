use clap::ValueEnum;
use strum::AsRefStr;

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum OutputFormat {
    Text,
    Json,
}

pub(crate) fn format_percent(value: &f64) -> String {
    format!("{:.2}%", value)
}

pub(crate) fn format_iec(size: &u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = *size as f64;
    let mut unit = &UNITS[0];

    for next_unit in &UNITS[1..] {
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
        unit = next_unit;
    }

    format!("{:.2}{}", size, unit)
}

pub(crate) fn format_si(size: &u64) -> String {
    const UNITS: [&str; 5] = ["", "K", "M", "G", "T"];
    let mut size = *size as f64;
    let mut unit = &UNITS[0];

    for next_unit in &UNITS[1..] {
        if size < 1000.0 {
            break;
        }
        size /= 1000.0;
        unit = next_unit;
    }

    format!("{:.0}{}", size, unit)
}

pub(crate) fn format_duration(secs: &u64) -> String {
    let mins = *secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours % 24 > 0 {
        parts.push(format!("{}h", hours % 24));
    }
    if mins % 60 > 0 {
        parts.push(format!("{}m", mins % 60));
    }
    if secs % 60 > 0 || parts.is_empty() {
        parts.push(format!("{}s", secs % 60));
    }

    parts.join("")
}

pub(crate) fn format_dynamic_precision(value: &f64) -> String {
    let decimal_part = value.fract();

    let precision = if decimal_part >= 0.1 {
        // If the decimal part is greater than or equal to the first decimal place, use 3 decimal places
        3
    } else {
        // Convert to string and find the position of the first significant digit
        let decimal_str = format!("{:}", decimal_part);
        if let Some(pos) = decimal_str.find(|c: char| c != '0' && c != '.') {
            pos - 1
        } else {
            // Integer case
            0
        }
    };

    format!("{:.precision$}s", *value, precision = precision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_si() {
        assert_eq!(format_si(&999), "999");
        assert_eq!(format_si(&1000), "1K");
        assert_eq!(format_si(&1500), "2K");
        assert_eq!(format_si(&1_000_000), "1M");
        assert_eq!(format_si(&1_500_000), "2M");
        assert_eq!(format_si(&1_000_000_000), "1G");
        assert_eq!(format_si(&1_500_000_000), "2G");
        assert_eq!(format_si(&1_000_000_000_000), "1T");
    }

    #[test]
    fn test_format_iec() {
        assert_eq!(format_iec(&1023), "1023.00B");
        assert_eq!(format_iec(&1024), "1.00KiB");
        assert_eq!(format_iec(&1536), "1.50KiB");
        assert_eq!(format_iec(&1_048_576), "1.00MiB");
        assert_eq!(format_iec(&1_572_864), "1.50MiB");
        assert_eq!(format_iec(&1_073_741_824), "1.00GiB");
        assert_eq!(format_iec(&1_610_612_736), "1.50GiB");
        assert_eq!(format_iec(&1_099_511_627_776), "1.00TiB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(&59), "59s");
        assert_eq!(format_duration(&60), "1m");
        assert_eq!(format_duration(&61), "1m1s");
        assert_eq!(format_duration(&3600), "1h");
        assert_eq!(format_duration(&3661), "1h1m1s");
        assert_eq!(format_duration(&86400), "1d");
        assert_eq!(format_duration(&90061), "1d1h1m1s");
    }
}
