use std::fs;
use std::path::Path;
use anyhow::Result;

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn format_number(num: u32) -> String {
    if num < 1000 {
        num.to_string()
    } else if num < 1_000_000 {
        format!("{:.1}K", num as f64 / 1000.0)
    } else {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    }
}

pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("${:.3}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(1_500_000), "1.5M");
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(format_cost(0.005), "$0.005");
        assert_eq!(format_cost(0.15), "$0.15");
        assert_eq!(format_cost(1.234), "$1.23");
    }
}