/// Binary units keep the value unambiguous (GiB = 1,073,741,824 bytes).
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    // Avoid displaying 1024.0 of the lower unit after rounding.
    if value >= 1023.95 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::format_file_size;

    #[test]
    fn formats_sizes_from_bytes_to_large_show_totals() {
        for (bytes, expected) in [
            (0, "0 B"),
            (1023, "1023 B"),
            (1024, "1.0 KiB"),
            (1_572_864, "1.5 MiB"),
            (5_368_709_120, "5.0 GiB"),
            (1_099_511_627_776, "1.0 TiB"),
            (1_048_575, "1.0 MiB"),
            (u64::MAX, "16.0 EiB"),
        ] {
            assert_eq!(format_file_size(bytes), expected);
        }
    }
}
