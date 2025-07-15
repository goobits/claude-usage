pub fn format_with_commas(num: u32) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    let mut count = 0;
    
    for ch in num_str.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
        count += 1;
    }
    
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_with_commas() {
        assert_eq!(format_with_commas(500), "500");
        assert_eq!(format_with_commas(1500), "1,500");
        assert_eq!(format_with_commas(1_500_000), "1,500,000");
    }
}