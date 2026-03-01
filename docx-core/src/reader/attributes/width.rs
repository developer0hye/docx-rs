use std::str::FromStr;

use crate::reader::OwnedAttribute;

use crate::types::*;

use super::super::errors::*;

/// Parse a numeric prefix from a string, ignoring any trailing non-numeric
/// characters (e.g. unit suffixes like "dxa" in Strict OOXML documents).
/// Returns 0.0 if the string contains no parseable numeric prefix.
fn parse_width_value(s: &str) -> f64 {
    let v = s.replace('%', "");
    // Try parsing the whole string first (fast path).
    if let Ok(f) = f64::from_str(&v) {
        return f;
    }
    // Find the longest numeric prefix (digits, '.', '-', '+') and parse that.
    let end = v
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+')
        .unwrap_or(v.len());
    if end == 0 {
        return 0.0;
    }
    f64::from_str(&v[..end]).unwrap_or(0.0)
}

pub fn read_width(attrs: &[OwnedAttribute]) -> Result<(isize, WidthType), ReaderError> {
    let mut w = 0;
    let mut width_type = WidthType::Auto;
    for a in attrs {
        let local_name = &a.name.local_name;
        if local_name == "type" {
            width_type = WidthType::from_str(&a.value)?;
        } else if local_name == "w" {
            w = parse_width_value(&a.value) as isize;
        }
    }
    Ok((w, width_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_integer() {
        assert_eq!(parse_width_value("1440"), 1440.0);
    }

    #[test]
    fn parse_plain_float() {
        assert_eq!(parse_width_value("14.5"), 14.5);
    }

    #[test]
    fn parse_value_with_dxa_suffix() {
        assert_eq!(parse_width_value("1440dxa"), 1440.0);
    }

    #[test]
    fn parse_value_with_percent() {
        assert_eq!(parse_width_value("50%"), 50.0);
    }

    #[test]
    fn parse_negative_value() {
        assert_eq!(parse_width_value("-100"), -100.0);
    }

    #[test]
    fn parse_empty_string() {
        assert_eq!(parse_width_value(""), 0.0);
    }

    #[test]
    fn parse_non_numeric_string() {
        assert_eq!(parse_width_value("abc"), 0.0);
    }

    #[test]
    fn parse_zero() {
        assert_eq!(parse_width_value("0"), 0.0);
    }
}
