//! Thin CLI over the `addrkit` library. All the actual logic lives in
//! `lib.rs`; this file's only job is turning argv into an `Address`
//! and printing whatever the library function returns.

use addrkit::{format_us_address, parse_us_address, validate, Address};
use std::env;
use std::process::ExitCode;

fn usage() -> String {
    "usage: addrkit <format|validate> [flags]\n\
     \x20\x20 or: addrkit parse <freeform address text>\n\
     \n\
     flags (format, validate):\n\
     \x20\x20--recipient <name>\n\
     \x20\x20--street1 <line>\n\
     \x20\x20--street2 <line>\n\
     \x20\x20--city <city>\n\
     \x20\x20--region <state or province>\n\
     \x20\x20--postal <postal code>\n\
     \x20\x20--country <country>\n\
     \n\
     examples:\n\
     \x20\x20addrkit format --recipient \"Jane Doe\" --street1 \"123 Main St\" \\\n\
     \x20\x20  --city Springfield --region IL --postal 62704\n\
     \x20\x20addrkit parse \"Jane Doe, 123 Main St, Springfield, IL 62704\""
        .to_string()
}

/// Turns `--flag value` pairs into an `Address`. Unknown flags are
/// reported as an error string; this is the only place in the binary
/// that deals with malformed input, keeping the library itself free
/// of argv concerns.
fn parse_address(args: &[String]) -> Result<Address, String> {
    let mut address = Address::default();
    let mut iter = args.iter();

    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("flag '{flag}' is missing a value"))?
            .clone();

        match flag.as_str() {
            "--recipient" => address.recipient = value,
            "--street1" => address.street1 = value,
            "--street2" => address.street2 = value,
            "--city" => address.city = value,
            "--region" => address.region = value,
            "--postal" => address.postal_code = value,
            "--country" => address.country = value,
            other => return Err(format!("unknown flag '{other}'")),
        }
    }

    Ok(address)
}

fn run(args: Vec<String>) -> Result<String, String> {
    let mut iter = args.into_iter();
    let command = iter.next().ok_or_else(usage)?;
    let rest: Vec<String> = iter.collect();

    match command.as_str() {
        "format" => {
            let address = parse_address(&rest)?;
            Ok(format_us_address(&address))
        }
        "validate" => {
            let address = parse_address(&rest)?;
            let issues = validate(&address);
            if issues.is_empty() {
                Ok("ok".to_string())
            } else {
                let lines: Vec<String> = issues.iter().map(|i| i.to_string()).collect();
                Ok(lines.join("\n"))
            }
        }
        "parse" => {
            if rest.is_empty() {
                return Err(format!(
                    "parse requires a freeform address argument\n\n{}",
                    usage()
                ));
            }
            // Args come pre-split by the shell; a caller passing an
            // unquoted address gets its words rejoined with spaces,
            // which loses the comma/newline structure the parser
            // relies on. Quoting the whole address is on them, same
            // as it would be for any other CLI that takes free text.
            let text = rest.join(" ");
            Ok(format_us_address(&parse_us_address(&text)))
        }
        other => Err(format!("unknown command '{other}'\n\n{}", usage())),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("{}", usage());
        return ExitCode::FAILURE;
    }

    match run(args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn run_formats_from_flags() {
        let output = run(args(&[
            "format",
            "--recipient",
            "Jane Doe",
            "--street1",
            "123 Main St",
            "--city",
            "Springfield",
            "--region",
            "IL",
            "--postal",
            "62704",
        ]))
        .unwrap();
        assert_eq!(output, "Jane Doe\n123 Main St\nSpringfield, IL 62704");
    }

    #[test]
    fn run_parses_freeform_text_into_a_formatted_block() {
        let output = run(args(&[
            "parse",
            "Jane Doe, 123 Main St, Springfield, IL 62704",
        ]))
        .unwrap();
        assert_eq!(output, "Jane Doe\n123 Main St\nSpringfield, IL 62704");
    }

    #[test]
    fn run_parse_without_text_is_an_error() {
        assert!(run(args(&["parse"])).is_err());
    }

    #[test]
    fn run_rejects_unknown_command() {
        assert!(run(args(&["bogus"])).is_err());
    }
}
