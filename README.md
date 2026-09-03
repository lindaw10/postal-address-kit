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
use addrkit::{format_us_address, validate, Address};

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

$ addrkit validate --recipient "Jane Doe" --street1 "123 Main St"
city is missing
region is missing
```

## Status

Early skeleton. US formatting and structural validation work; parsing
freeform address text into fields, international formats, and a
reference table of state/province codes are not built yet (see below).

## Roadmap

- Freeform address parser (single string -> `Address`)
- State/province code table with format-aware validation
- Basic international address formats beyond the US
- `--json` output mode for the CLI
- Property-based tests for the normalization functions

## License

MIT, see [LICENSE](LICENSE).
