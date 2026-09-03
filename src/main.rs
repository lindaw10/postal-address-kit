//! Thin CLI over the `addrkit` library. All the actual logic lives in
//! `lib.rs`; this file's only job is turning argv into an `Address`
//! and printing whatever the library function returns.

use addrkit::{format_us_address, validate, Address};
use std::env;
use std::process::ExitCode;

fn usage() -> String {
    "usage: addrkit <format|validate> [flags]\n\
     \n\
     flags:\n\
     \x20\x20--recipient <name>\n\
     \x20\x20--street1 <line>\n\
     \x20\x20--street2 <line>\n\
     \x20\x20--city <city>\n\
     \x20\x20--region <state or province>\n\
     \x20\x20--postal <postal code>\n\
     \x20\x20--country <country>\n\
     \n\
     example:\n\
     \x20\x20addrkit format --recipient \"Jane Doe\" --street1 \"123 Main St\" \\\n\
     \x20\x20  --city Springfield --region IL --postal 62704"
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
    let address = parse_address(&rest)?;

    match command.as_str() {
        "format" => Ok(format_us_address(&address)),
        "validate" => {
            let issues = validate(&address);
            if issues.is_empty() {
                Ok("ok".to_string())
            } else {
                let lines: Vec<String> = issues.iter().map(|i| i.to_string()).collect();
                Ok(lines.join("\n"))
            }
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
