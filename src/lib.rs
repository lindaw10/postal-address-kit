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

/// The states, DC, and inhabited territories the USPS assigns a
/// two-letter code to, paired with their full names.
///
/// Military "state" codes (AA, AE, AP) are deliberately left out: they
/// don't have a single associated place name, so they don't fit the
/// name-or-abbreviation lookup [`normalize_us_region`] does.
const US_STATES: &[(&str, &str)] = &[
    ("Alabama", "AL"),
    ("Alaska", "AK"),
    ("Arizona", "AZ"),
    ("Arkansas", "AR"),
    ("California", "CA"),
    ("Colorado", "CO"),
    ("Connecticut", "CT"),
    ("Delaware", "DE"),
    ("District of Columbia", "DC"),
    ("Florida", "FL"),
    ("Georgia", "GA"),
    ("Hawaii", "HI"),
    ("Idaho", "ID"),
    ("Illinois", "IL"),
    ("Indiana", "IN"),
    ("Iowa", "IA"),
    ("Kansas", "KS"),
    ("Kentucky", "KY"),
    ("Louisiana", "LA"),
    ("Maine", "ME"),
    ("Maryland", "MD"),
    ("Massachusetts", "MA"),
    ("Michigan", "MI"),
    ("Minnesota", "MN"),
    ("Mississippi", "MS"),
    ("Missouri", "MO"),
    ("Montana", "MT"),
    ("Nebraska", "NE"),
    ("Nevada", "NV"),
    ("New Hampshire", "NH"),
    ("New Jersey", "NJ"),
    ("New Mexico", "NM"),
    ("New York", "NY"),
    ("North Carolina", "NC"),
    ("North Dakota", "ND"),
    ("Ohio", "OH"),
    ("Oklahoma", "OK"),
    ("Oregon", "OR"),
    ("Pennsylvania", "PA"),
    ("Rhode Island", "RI"),
    ("South Carolina", "SC"),
    ("South Dakota", "SD"),
    ("Tennessee", "TN"),
    ("Texas", "TX"),
    ("Utah", "UT"),
    ("Vermont", "VT"),
    ("Virginia", "VA"),
    ("Washington", "WA"),
    ("West Virginia", "WV"),
    ("Wisconsin", "WI"),
    ("Wyoming", "WY"),
    ("American Samoa", "AS"),
    ("Guam", "GU"),
    ("Northern Mariana Islands", "MP"),
    ("Puerto Rico", "PR"),
    ("U.S. Virgin Islands", "VI"),
];

/// Normalizes a US state, DC, or territory name or abbreviation to its
/// canonical two-letter code.
///
/// Accepts either form case-insensitively - `"illinois"`, `"Illinois"`,
/// and `"il"` all normalize to `"IL"` - since freeform input and form
/// fields disagree about which one to use. Returns `None` if the input
/// isn't a two-letter code or full name in the state table, which is
/// how [`validate`] tells a real US region apart from a typo or a
/// foreign province.
pub fn normalize_us_region(input: &str) -> Option<String> {
    let cleaned = normalize_whitespace(input);
    if cleaned.is_empty() {
        return None;
    }
    let upper = cleaned.to_uppercase();

    if cleaned.chars().count() == 2 {
        return US_STATES
            .iter()
            .find(|(_, abbr)| *abbr == upper)
            .map(|(_, abbr)| abbr.to_string());
    }

    US_STATES
        .iter()
        .find(|(name, _)| name.to_uppercase() == upper)
        .map(|(_, abbr)| abbr.to_string())
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
    InvalidRegion(String),
    InvalidPostalCode(String),
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationIssue::MissingRecipient => write!(f, "recipient is missing"),
            ValidationIssue::MissingStreet => write!(f, "street1 is missing"),
            ValidationIssue::MissingCity => write!(f, "city is missing"),
            ValidationIssue::MissingRegion => write!(f, "region is missing"),
            ValidationIssue::InvalidRegion(raw) => {
                write!(f, "region '{raw}' is not a recognized US state, DC, or territory")
            }
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
/// the postal code has a plausible shape, whether the region is a
/// real US state/DC/territory), not whether the address actually
/// exists. That kind of lookup needs a data source and does not
/// belong in a pure function.
///
/// The region check only runs for addresses that look domestic (an
/// empty or US-flavored `country` field): a blank country is the
/// common case for addresses that never had one to begin with, and
/// treating it as domestic matches [`format_us_address`], which also
/// only adds a country line for non-US addresses. Addresses with a
/// foreign `country` skip the check entirely, since the state table
/// has nothing to say about a Canadian province or a UK county.
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

    let region = normalize_whitespace(&address.region);
    let country = normalize_whitespace(&address.country);
    if region.is_empty() {
        issues.push(ValidationIssue::MissingRegion);
    } else if (country.is_empty() || is_domestic_us(&country))
        && normalize_us_region(&region).is_none()
    {
        issues.push(ValidationIssue::InvalidRegion(region));
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
/// country line is omitted for domestic addresses. A region that
/// matches a known US state, DC, or territory name is canonicalized
/// to its two-letter code (`"Illinois"` becomes `"IL"`); anything
/// else, including a region that's already an unrecognized code, is
/// passed through unchanged. This never fails; pass the result
/// through [`validate`] first if you need to know whether the input
/// was actually complete.
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
    let region_raw = normalize_whitespace(&address.region);
    let region = normalize_us_region(&region_raw).unwrap_or(region_raw);
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

/// Parses a freeform US address string into an [`Address`].
///
/// Accepts either the multi-line block that [`format_us_address`]
/// produces or a single comma-separated line - input is split on both
/// newlines and commas, so callers don't need to know or care which
/// style they're feeding in. The last segment is read as
/// "region [postal code]", the segment before that as the city, and
/// whatever is left at the front as recipient and street lines: a
/// leading segment that starts with a digit is assumed to be a street
/// line rather than a recipient name.
///
/// This is a heuristic splitter, not a real address-parsing engine -
/// it has no knowledge of actual street or place names, so ambiguous
/// input (a single segment with no other context) gets assigned
/// somewhere reasonable rather than rejected. Run the result through
/// [`validate`] if you need to confirm the required fields actually
/// came out non-empty.
pub fn parse_us_address(input: &str) -> Address {
    let mut segments: Vec<String> = input
        .split(|c| c == '\n' || c == ',')
        .map(normalize_whitespace)
        .filter(|s| !s.is_empty())
        .collect();

    let mut address = Address::default();

    if segments.is_empty() {
        return address;
    }

    if is_domestic_us(segments.last().unwrap()) {
        address.country = segments.pop().unwrap();
    }

    if segments.is_empty() {
        return address;
    }

    let locality = segments.pop().unwrap();
    let mut tokens: Vec<&str> = locality.split_whitespace().collect();
    if let Some(last_token) = tokens.last() {
        if last_token.chars().any(|c| c.is_ascii_digit()) {
            let raw_postal = tokens.pop().unwrap();
            address.postal_code =
                normalize_us_postal_code(raw_postal).unwrap_or_else(|| raw_postal.to_string());
        }
    }
    address.region = tokens.join(" ");

    if let Some(city) = segments.pop() {
        address.city = city;
    }

    if segments.is_empty() {
        return address;
    }

    if starts_with_digit(&segments[0]) {
        address.street1 = segments.remove(0);
    } else {
        address.recipient = segments.remove(0);
        if !segments.is_empty() {
            address.street1 = segments.remove(0);
        }
    }
    if !segments.is_empty() {
        address.street2 = segments.join(", ");
    }

    address
}

/// Reports whether a segment's first word starts with a digit, the
/// signal [`parse_us_address`] uses to tell a street line ("123 Main
/// St") apart from a recipient name at the front of the input.
fn starts_with_digit(segment: &str) -> bool {
    segment
        .split_whitespace()
        .next()
        .and_then(|word| word.chars().next())
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
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
    fn normalize_us_region_accepts_abbreviation_case_insensitively() {
        assert_eq!(normalize_us_region("il"), Some("IL".to_string()));
        assert_eq!(normalize_us_region("Il"), Some("IL".to_string()));
    }

    #[test]
    fn normalize_us_region_accepts_full_name_case_insensitively() {
        assert_eq!(normalize_us_region("illinois"), Some("IL".to_string()));
        assert_eq!(normalize_us_region("  New   York "), Some("NY".to_string()));
    }

    #[test]
    fn normalize_us_region_accepts_dc_and_territories() {
        assert_eq!(
            normalize_us_region("District of Columbia"),
            Some("DC".to_string())
        );
        assert_eq!(normalize_us_region("pr"), Some("PR".to_string()));
        assert_eq!(normalize_us_region("Puerto Rico"), Some("PR".to_string()));
    }

    #[test]
    fn normalize_us_region_rejects_unknown_values() {
        assert_eq!(normalize_us_region("Ontario"), None);
        assert_eq!(normalize_us_region("ZZ"), None);
        assert_eq!(normalize_us_region(""), None);
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
    fn format_us_address_canonicalizes_full_state_name() {
        let mut addr = sample();
        addr.region = "Illinois".to_string();
        let formatted = format_us_address(&addr);
        assert_eq!(
            formatted,
            "Jane Doe\n123 Main St\nApt 4B\nSpringfield, IL 62704"
        );
    }

    #[test]
    fn format_us_address_passes_through_unrecognized_region() {
        let mut addr = sample();
        addr.region = "Ontario".to_string();
        let formatted = format_us_address(&addr);
        assert!(formatted.contains("Springfield, Ontario 62704"));
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
    fn validate_flags_unrecognized_region_for_domestic_address() {
        let mut addr = sample();
        addr.region = "Not A State".to_string();
        let issues = validate(&addr);
        assert_eq!(
            issues,
            vec![ValidationIssue::InvalidRegion("Not A State".to_string())]
        );
    }

    #[test]
    fn validate_accepts_full_state_name_as_region() {
        let mut addr = sample();
        addr.region = "Illinois".to_string();
        assert!(validate(&addr).is_empty());
    }

    #[test]
    fn validate_skips_region_check_for_foreign_address() {
        let mut addr = sample();
        addr.region = "Ontario".to_string();
        addr.country = "Canada".to_string();
        assert!(validate(&addr).is_empty());
    }

    #[test]
    fn validate_checks_region_when_country_is_blank() {
        let mut addr = sample();
        addr.region = "Not A State".to_string();
        addr.country = "".to_string();
        let issues = validate(&addr);
        assert_eq!(
            issues,
            vec![ValidationIssue::InvalidRegion("Not A State".to_string())]
        );
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

    #[test]
    fn parse_us_address_reads_multiline_block() {
        let parsed = parse_us_address("Jane Doe\n123 Main St\nApt 4B\nSpringfield, IL 62704");
        assert_eq!(parsed.recipient, "Jane Doe");
        assert_eq!(parsed.street1, "123 Main St");
        assert_eq!(parsed.street2, "Apt 4B");
        assert_eq!(parsed.city, "Springfield");
        assert_eq!(parsed.region, "IL");
        assert_eq!(parsed.postal_code, "62704");
        assert_eq!(parsed.country, "");
    }

    #[test]
    fn parse_us_address_reads_single_comma_separated_line() {
        let parsed =
            parse_us_address("Jane Doe, 123 Main St, Apt 4B, Springfield, IL 62704, USA");
        assert_eq!(parsed.recipient, "Jane Doe");
        assert_eq!(parsed.street1, "123 Main St");
        assert_eq!(parsed.street2, "Apt 4B");
        assert_eq!(parsed.city, "Springfield");
        assert_eq!(parsed.region, "IL");
        assert_eq!(parsed.postal_code, "62704");
        assert_eq!(parsed.country, "USA");
    }

    #[test]
    fn parse_us_address_normalizes_zip_plus_four_and_junk_whitespace() {
        let parsed = parse_us_address("123  Main St\nSpringfield,   IL   62704-1234");
        assert_eq!(parsed.street1, "123 Main St");
        assert_eq!(parsed.postal_code, "62704-1234");
    }

    #[test]
    fn parse_us_address_without_recipient_treats_leading_digits_as_street() {
        let parsed = parse_us_address("123 Main St, Springfield, IL 62704");
        assert_eq!(parsed.recipient, "");
        assert_eq!(parsed.street1, "123 Main St");
        assert_eq!(parsed.street2, "");
    }

    #[test]
    fn parse_us_address_handles_city_region_postal_only() {
        let parsed = parse_us_address("Springfield, IL 62704");
        assert_eq!(parsed.recipient, "");
        assert_eq!(parsed.street1, "");
        assert_eq!(parsed.city, "Springfield");
        assert_eq!(parsed.region, "IL");
        assert_eq!(parsed.postal_code, "62704");
    }

    #[test]
    fn parse_us_address_of_empty_string_is_default() {
        assert_eq!(parse_us_address(""), Address::default());
        assert_eq!(parse_us_address("   \n , "), Address::default());
    }

    #[test]
    fn parse_us_address_round_trips_through_format() {
        let original = sample();
        let block = format_us_address(&original);
        let parsed = parse_us_address(&block);
        assert_eq!(parsed.recipient, original.recipient.trim());
        assert_eq!(parsed.street1, original.street1);
        assert_eq!(parsed.street2, original.street2);
        assert_eq!(parsed.city, original.city);
        assert_eq!(parsed.region, original.region);
        assert_eq!(parsed.postal_code, original.postal_code);
    }
}
