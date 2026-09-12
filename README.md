# addrkit

A small Rust library for cleaning up, formatting, and validating postal
addresses, with a thin CLI on top.

## Why

Addresses that come from web forms, spreadsheets, or pasted-in PDFs are
rarely clean: extra whitespace, ZIP codes with stray characters, a
country field that's sometimes "US" and sometimes "United States". Most
of that mess needs the same handful of fixes every time. `addrkit`
collects those fixes into small, pure functions that are easy to test
and easy to call from a bigger program, plus a CLI for quick one-off
use.

This is not an address *verification* service - it doesn't know
whether 123 Main St actually exists. It only checks structure (are the
required fields present, does the postal code have a plausible shape)
and produces a correctly formatted output block.

## Design

Every public function in the library is pure: same input, same output,
no I/O, no shared state. That makes them trivial to unit test and safe
to reuse in a web handler, a batch script, or anywhere else, without
worrying about side effects. The CLI is a thin wrapper that turns argv
into an `Address` and prints whatever the library returns - it has no
logic of its own worth testing separately.

## Library usage

```rust
use addrkit::{format_address, format_us_address, parse_us_address, validate, Address};

let address = Address {
    recipient: "Jane Doe".to_string(),
    street1: "123 Main St".to_string(),
    street2: "Apt 4B".to_string(),
    city: "Springfield".to_string(),
    region: "IL".to_string(),
    postal_code: "62704".to_string(),
    country: "US".to_string(),
};

assert!(validate(&address).is_empty());

println!("{}", format_us_address(&address));
// Jane Doe
// 123 Main St
// Apt 4B
// Springfield, IL 62704

let parsed = parse_us_address("Jane Doe, 123 Main St, Apt 4B, Springfield, IL 62704");
assert_eq!(parsed.city, address.city);
assert_eq!(parsed.postal_code, address.postal_code);

let uk_address = Address {
    street1: "10 Downing Street".to_string(),
    city: "London".to_string(),
    postal_code: "SW1A 2AA".to_string(),
    country: "United Kingdom".to_string(),
    ..Address::default()
};

// format_address picks the US or international layout based on
// the country field; call format_us_address or
// format_international_address directly if you already know which
// one you want.
println!("{}", format_address(&uk_address));
// 10 Downing Street
// LONDON SW1A 2AA
// UNITED KINGDOM
```

## CLI usage

```sh
$ addrkit format \
    --recipient "Jane Doe" \
    --street1 "123 Main St" \
    --street2 "Apt 4B" \
    --city Springfield \
    --region IL \
    --postal "62704"
Jane Doe
123 Main St
Apt 4B
Springfield, IL 62704

$ addrkit format --street1 "10 Downing Street" --city London \
    --postal "SW1A 2AA" --country "United Kingdom"
10 Downing Street
LONDON SW1A 2AA
UNITED KINGDOM

$ addrkit validate --recipient "Jane Doe" --street1 "123 Main St"
city is missing
region is missing

$ addrkit parse "Jane Doe, 123 Main St, Apt 4B, Springfield, IL 62704"
Jane Doe
123 Main St
Apt 4B
Springfield, IL 62704
```

`parse` splits on both commas and newlines, so a pasted multi-line
block works the same way. It's a heuristic, not a real parsing engine:
it doesn't know street or place names, it just uses field order and a
few shape checks (does a line start with a digit, does it end in
something with digits in it) to guess where recipient, street, city,
region, and postal code start and stop.

## Status

Early skeleton. US formatting, a generic international layout,
structural validation, freeform parsing, and a US state/DC/territory
code table are in place.

`validate` checks the `region` field against that table - `"Illinois"`,
`"illinois"`, and `"IL"` all pass, anything else doesn't - but only for
addresses that look domestic (a blank or US `country` field). A region
paired with a non-US country is left alone, since the table has nothing
to say about a Canadian province or a UK county.

The international layout (`format_international_address`, or
`format_address` when the country isn't domestic US) is deliberately
basic: one line each for recipient, street1, and street2, then a
locality line (city, region, postal code) and a country line, both in
capitals. Real per-country conventions - postal code before the city
in some countries, no region line at all in others - aren't modeled.

## Roadmap

- `--json` output mode for the CLI
- Property-based tests for the normalization functions

## License

MIT, see [LICENSE](LICENSE).
