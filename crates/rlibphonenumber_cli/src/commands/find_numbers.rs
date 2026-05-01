use std::{error::Error, str::FromStr};

use argh::FromArgs;
use rlibphonenumber::{
    enums::Region,
    phonenumber_matcher::{Leniency, PHONE_MATCHER_FACTORY},
};

use crate::sources::Source;

fn parse_leniency(s: &str) -> Result<Leniency, String> {
    Leniency::from_str(s).map_err(|err| err.to_string())
}

fn parse_region(s: &str) -> Result<Region, String> {
    Region::from_str(s).map_err(|err| err.to_string())
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "find-numbers")]
/// Extracts and deduplicates phone numbers from a text source using a sliding window.
pub struct FindNumbersCommand {
    /// input source. Supports:
    /// - Local file paths (e.g., path/to/file.txt)
    /// - HTTP/HTTPS URLs (e.g., https://example.com/data.txt)
    /// - SSH paths (e.g., user@host:/path/to/file)
    /// - Git repositories (e.g., git://github.com/user/repo.git?file=data.txt&branch=main)
    #[argh(positional)]
    pub input: String,

    #[argh(option, from_str_fn(parse_leniency), default = "Leniency::Valid")]
    /// parser's leniency
    pub leniency: Leniency,

    #[argh(option, default = "u64::MAX")]
    /// max tries for parser to make trying to extract phone number per window
    pub max_tries: u64,

    /// default region code (e.g., "US", "GB") to assume for phone numbers without international prefixes.
    #[argh(option)]
    pub region: Option<Region>,

    /// size of the sliding window in bytes (default: 65536)
    #[argh(option, default = "65536")]
    pub window_size: usize,

    /// overlap size in bytes to prevent splitting numbers across chunks (default: 1024)
    #[argh(option, default = "1024")]
    pub overlap: usize,
}

/// CLI execution entry point: Handles file reading, sliding window search, and outputting results.
pub fn execute(options: FindNumbersCommand) -> Result<(), Box<dyn Error>> {
    let source: Source = options.input.parse()?;
    let filename = source.file_name().unwrap_or_else(|| options.input.clone());

    println!(
        "Searching for phone numbers in '{}' (window: {}, overlap: {})...",
        filename, options.window_size, options.overlap
    );

    let unique_numbers = source.search_phone_numbers(
        options.window_size,
        options.overlap,
        |text_chunk, yield_match| {
            let matcher = PHONE_MATCHER_FACTORY
                .matcher_builder(text_chunk)
                .preferred_region(options.region)
                .leniency(options.leniency)
                .max_tries(options.max_tries)
                .build();

            for phone_match in matcher {
                yield_match(phone_match);
            }
        },
    )?;

    println!("Success! Found {} unique numbers.", unique_numbers.len());

    // Optional: Print a summary of the found numbers
    for found in &unique_numbers {
        // For formatting, we typically use the E164 format (+1234567890)
        let formatted = found
            .number
            .format_as(rlibphonenumber::PhoneNumberFormat::E164);

        println!("- Matched: {} (Length: {})", formatted, found.len);
    }

    Ok(())
}
