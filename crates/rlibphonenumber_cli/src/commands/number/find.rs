use std::{
    error::Error,
    io::{self, BufWriter, Write},
    str::FromStr,
};

use argh::FromArgs;

use rlibphonenumber::phonenumber_matcher::{Leniency, PhoneNumberMatcherFactory};
use rlibphonenumber::{PhoneNumberFormat, PhoneNumberUtil, enums::Region};

use crate::sources::{FoundToken, SearchNumbers, Source};

use super::parse::print_formatted_output;
use super::{BUF_CAPACITY, parse_leniency};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "find")]
/// Extracts and streams phone numbers from a text source.
pub struct FindCommand {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: String,

    /// output mode: 'plaintext', 'json', or 'wide'
    #[argh(option, short = 'o', default = "String::from(\"plaintext\")")]
    pub output: String,

    /// phone number format: 'e164', 'international', 'national', 'rfc3966'
    #[argh(option, short = 'f', default = "String::from(\"e164\")")]
    pub format: String,

    /// parser's leniency
    #[argh(
        option,
        short = 'l',
        from_str_fn(parse_leniency),
        default = "Leniency::Valid"
    )]
    pub leniency: Leniency,

    /// max tries for parser
    #[argh(option, short = 't', default = "u64::MAX")]
    pub max_tries: u64,

    /// size of the sliding window in bytes
    #[argh(option, short = 'w', default = "65536")]
    pub window_size: usize,

    /// overlap size in bytes
    #[argh(option, short = 'v', default = "1024")]
    pub overlap: usize,
}

pub fn execute(
    options: FindCommand,
    util: &PhoneNumberUtil,
    region: Option<Region>,
) -> Result<(), Box<dyn Error>> {
    let source: Source = options.input.parse()?;

    let stdout_raw = io::stdout();
    let mut stdout = BufWriter::with_capacity(BUF_CAPACITY, stdout_raw.lock());
    let format_fmt =
        PhoneNumberFormat::from_str(&options.format).unwrap_or(PhoneNumberFormat::E164);

    let factory = PhoneNumberMatcherFactory::new_for_util(util);

    source.search_phone_numbers(
        options.window_size,
        options.overlap,
        |text_chunk, yield_match| {
            let matcher = factory
                .matcher_builder(text_chunk)
                .preferred_region(region)
                .leniency(options.leniency)
                .max_tries(options.max_tries)
                .build();

            for phone_match in matcher {
                yield_match(phone_match);
            }
        },
        |token| match token {
            FoundToken::Phone(found) => {
                print_formatted_output(
                    &mut stdout,
                    util,
                    &found.number,
                    None,
                    format_fmt,
                    &options.format,
                    region,
                    &options.output,
                );
            }
            FoundToken::NoPhone(_) => {}
        },
    )?;

    stdout.flush()?;
    Ok(())
}
