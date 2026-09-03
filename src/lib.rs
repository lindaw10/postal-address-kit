//! Normalization, formatting, and validation for postal addresses.
//!
//! Every public function here is pure: given the same input it always
//! produces the same output, and none of them touch the filesystem,
//! the network, or any other kind of ambient state. That is what makes
//! them easy to test and safe to call from anywhere, including a CLI
//! that also has to deal with args and stdin.

use std::fmt;

/// A postal address broken into the fields most mail systems expect.
///
/// Fields are plain `String`s rather than an enum of "known good"
/// values, because addresses arrive messy (extra whitespace, mixed
/// case, partial ZIP+4 codes) and the library's job is to clean that
/// up, not to reject it before the caller gets a chance to look at it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    pub recipient: String,
    pub street1: String,
    pub street2: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
}

/// Collapses runs of whitespace to single spaces and trims the ends.
///
/// Addresses pasted from PDFs and web forms routinely carry double
/// spaces, tabs, or trailing newlines; this is the one normalization
/// nearly every other function here depends on.
pub fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normalizes a US postal code to `NNNNN` or `NNNNN-NNNN`.
///
/// Non-digit characters (spaces, stray hyphens) are stripped before
/// counting, so `"12345 - 6789"` and `"123456789"` both normalize the
/// same way. Returns `None` if the digit count doesn't match either
/// a 5-digit or a ZIP+4 code.
pub fn normalize_us_postal_code(input: &str) -> Option<String> {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.len() {
        5 => Some(digits),
        9 => Some(format!("{}-{}", &digits[0..5], &digits[5..9])),
        _ => None,
    }
}

/// Reports whether a country field refers to the United States.
///
/// Used by [`format_us_address`] to decide whether a trailing country
/// line is needed: domestic mail conventionally omits it.
fn is_domestic_us(country: &str) -> bool {
    matches!(
        normalize_whitespace(country).to_uppercase().as_str(),
        "US" | "USA" | "U.S." | "U.S.A." | "UNITED STATES" | "UNITED STATES OF AMERICA"
    )
}

/// A single problem found by [`validate`].
///
/// Kept as a small enum rather than raw strings so callers (including
/// the CLI) can match on the kind of problem instead of parsing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    MissingRecipient,
    MissingStreet,
    MissingCity,
    MissingRegion,
    InvalidPostalCode(String),
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationIssue::MissingRecipient => write!(f, "recipient is missing"),
            ValidationIssue::MissingStreet => write!(f, "street1 is missing"),
            ValidationIssue::MissingCity => write!(f, "city is missing"),
            ValidationIssue::MissingRegion => write!(f, "region is missing"),
            ValidationIssue::InvalidPostalCode(raw) => {
                write!(f, "postal code '{raw}' is not a valid 5 or 9 digit US code")
            }
        }
    }
}

/// Checks an address for the problems that would keep it from being
/// deliverable, without trying to guess at fixes.
///
/// This only validates structure (which fields are present, whether
/// the postal code has a plausible shape), not whether the address
/// actually exists. That kind of lookup needs a data source and does
/// not belong in a pure function.
pub fn validate(address: &Address) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if normalize_whitespace(&address.recipient).is_empty() {
        issues.push(ValidationIssue::MissingRecipient);
    }
    if normalize_whitespace(&address.street1).is_empty() {
        issues.push(ValidationIssue::MissingStreet);
    }
    if normalize_whitespace(&address.city).is_empty() {
        issues.push(ValidationIssue::MissingCity);
    }
    if normalize_whitespace(&address.region).is_empty() {
        issues.push(ValidationIssue::MissingRegion);
    }

    let postal = normalize_whitespace(&address.postal_code);
    if !postal.is_empty() && normalize_us_postal_code(&postal).is_none() {
        issues.push(ValidationIssue::InvalidPostalCode(postal));
    }

    issues
}

/// Renders an address as the multi-line block a US envelope expects:
///
/// ```text
/// Jane Doe
/// 123 Main St
/// Apt 4B
/// Springfield, IL 62704
/// ```
///
/// Blank fields are skipped rather than left as empty lines, and the
/// country line is omitted for domestic addresses. This never fails;
/// pass the result through [`validate`] first if you need to know
/// whether the input was actually complete.
pub fn format_us_address(address: &Address) -> String {
    let mut lines = Vec::new();

    let recipient = normalize_whitespace(&address.recipient);
    if !recipient.is_empty() {
        lines.push(recipient);
    }

    let street1 = normalize_whitespace(&address.street1);
    if !street1.is_empty() {
        lines.push(street1);
    }

    let street2 = normalize_whitespace(&address.street2);
    if !street2.is_empty() {
        lines.push(street2);
    }

    let city = normalize_whitespace(&address.city);
    let region = normalize_whitespace(&address.region);
    let postal_raw = normalize_whitespace(&address.postal_code);
    let postal = normalize_us_postal_code(&postal_raw).unwrap_or(postal_raw);

    let mut city_line = String::new();
    if !city.is_empty() {
        city_line.push_str(&city);
    }
    if !region.is_empty() {
        if !city_line.is_empty() {
            city_line.push_str(", ");
        }
        city_line.push_str(&region);
    }
    if !postal.is_empty() {
        if !city_line.is_empty() {
            city_line.push(' ');
        }
        city_line.push_str(&postal);
    }
    if !city_line.is_empty() {
        lines.push(city_line);
    }

    let country = normalize_whitespace(&address.country);
    if !country.is_empty() && !is_domestic_us(&country) {
        lines.push(country.to_uppercase());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Address {
        Address {
            recipient: "  Jane   Doe ".to_string(),
            street1: "123 Main St".to_string(),
            street2: "Apt 4B".to_string(),
            city: "Springfield".to_string(),
            region: "IL".to_string(),
            postal_code: "62704".to_string(),
            country: "US".to_string(),
        }
    }

    #[test]
    fn normalize_whitespace_collapses_and_trims() {
        assert_eq!(normalize_whitespace("  a   b\tc\n"), "a b c");
        assert_eq!(normalize_whitespace(""), "");
    }

    #[test]
    fn normalize_us_postal_code_accepts_five_digit() {
        assert_eq!(normalize_us_postal_code("62704"), Some("62704".to_string()));
    }

    #[test]
    fn normalize_us_postal_code_accepts_zip_plus_four_with_junk() {
        assert_eq!(
            normalize_us_postal_code("62704 - 1234"),
            Some("62704-1234".to_string())
        );
    }

    #[test]
    fn normalize_us_postal_code_rejects_bad_length() {
        assert_eq!(normalize_us_postal_code("1234"), None);
        assert_eq!(normalize_us_postal_code(""), None);
    }

    #[test]
    fn format_us_address_produces_expected_block() {
        let formatted = format_us_address(&sample());
        assert_eq!(
            formatted,
            "Jane Doe\n123 Main St\nApt 4B\nSpringfield, IL 62704"
        );
    }

    #[test]
    fn format_us_address_omits_domestic_country_and_blank_fields() {
        let mut addr = sample();
        addr.street2 = "".to_string();
        addr.country = "United States".to_string();
        let formatted = format_us_address(&addr);
        assert_eq!(formatted, "Jane Doe\n123 Main St\nSpringfield, IL 62704");
    }

    #[test]
    fn format_us_address_keeps_foreign_country() {
        let mut addr = sample();
        addr.country = "Canada".to_string();
        let formatted = format_us_address(&addr);
        assert!(formatted.ends_with("\nCANADA"));
    }

    #[test]
    fn validate_flags_missing_required_fields() {
        let issues = validate(&Address::default());
        assert!(issues.contains(&ValidationIssue::MissingRecipient));
        assert!(issues.contains(&ValidationIssue::MissingStreet));
        assert!(issues.contains(&ValidationIssue::MissingCity));
        assert!(issues.contains(&ValidationIssue::MissingRegion));
    }

    #[test]
    fn validate_flags_malformed_postal_code() {
        let mut addr = sample();
        addr.postal_code = "abc".to_string();
        let issues = validate(&addr);
        assert_eq!(
            issues,
            vec![ValidationIssue::InvalidPostalCode("abc".to_string())]
        );
    }

    #[test]
    fn validate_accepts_complete_address() {
        assert!(validate(&sample()).is_empty());
    }
}
