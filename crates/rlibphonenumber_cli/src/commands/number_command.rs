use std::{
    error::Error,
    io::{self, BufWriter, Write},
    str::FromStr,
    sync::Arc,
};

use argh::FromArgs;
use hmac::{Hmac, KeyInit};
use prost::Message;
use rand::Rng;
use serde_json::json;

use rlibphonenumber::{
    PhoneMetadataCollection, PhoneNumber, PhoneNumberFormat, PhoneNumberUtil,
    enums::Region,
    interfaces::PhoneHasher,
    phonenumber_mask::{MaskDigitsConfig, PhoneMacHasher, PhoneMaskUtil},
    phonenumber_matcher::{Leniency, PhoneNumberMatcherFactory},
};
use sha2::Sha256;

use crate::sources::{FoundToken, ReadSource, SearchNumbers, Source};

const BUF_CAPACITY: usize = 256 * 1024;

#[cold]
#[inline(never)]
fn handle_io_err(e: io::Error) {
    match e.kind() {
        io::ErrorKind::BrokenPipe => std::process::exit(0),
        _ => {
            panic!("I/O Operation failed: {}", e);
        }
    }
}

macro_rules! handle_pipe {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => handle_io_err(e),
        }
    };
}

macro_rules! cli_write_str {
    ($dst:expr, $str:expr) => {
        handle_pipe!(($dst).write_all(($str).as_bytes()))
    };
}

macro_rules! cli_writeln {
    ($dst:expr, $($arg:tt)*) => {
        handle_pipe!(writeln!($dst, $($arg)*))
    };
}

fn parse_leniency(s: &str) -> Result<Leniency, String> {
    Leniency::from_str(s).map_err(|err| err.to_string())
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "number")]
/// Operations with phone numbers: parse, find, mask
pub struct NumberCommand {
    /// default region for given phone
    #[argh(option, short = 'r')]
    pub region: Option<String>,

    /// custom metadata source (file path, URL, etc.)
    #[argh(option, short = 'm')]
    pub metadata: Option<String>,

    #[argh(subcommand)]
    pub action: NumberAction,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum NumberAction {
    Parse(ParseCommand),
    Find(FindCommand),
    Mask(MaskCommand),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "parse")]
/// Get parsed information about a specific phone number.
pub struct ParseCommand {
    /// the phone number to parse (e.g. +1234567890)
    #[argh(positional)]
    pub number: String,

    /// output mode: 'plaintext', 'json', or 'wide'
    #[argh(option, short = 'o', default = "String::from(\"plaintext\")")]
    pub output: String,

    /// phone number format: 'e164', 'international', 'national', 'rfc3966'
    #[argh(option, short = 'f', default = "String::from(\"e164\")")]
    pub format: String,
}

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

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "mask")]
/// Streams text from a source, replacing phone numbers with masked versions.
pub struct MaskCommand {
    /// leniency to search number
    #[argh(
        option,
        short = 'l',
        from_str_fn(parse_leniency),
        default = "Leniency::Valid"
    )]
    pub leniency: Leniency,

    /// max tries to search number
    #[argh(option, short = 't', default = "u64::MAX")]
    pub max_tries: u64,

    /// search window size
    #[argh(option, short = 'w', default = "65536")]
    pub window_size: usize,

    /// search window overlap
    #[argh(option, short = 'v', default = "1024")]
    pub overlap: usize,

    #[argh(subcommand)]
    pub mask_type: MaskType,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum MaskType {
    Constant(ConstantMask),
    Token(TokenMask),
    Hash(HashMask),
    Mask(FormatMask),
}

impl MaskType {
    fn input_str(&self) -> &str {
        match self {
            MaskType::Constant(c) => &c.input,
            MaskType::Token(t) => &t.input,
            MaskType::Hash(h) => &h.input,
            MaskType::Mask(m) => &m.input,
        }
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "constant")]
/// Replace with a constant string.
pub struct ConstantMask {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: String,
    /// the constant string to replace the phone number with
    #[argh(positional)]
    pub value: String,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "token")]
/// Replace with a semantic token.
pub struct TokenMask {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: String,
    /// whether to exclude the hash in the token
    #[argh(switch, short = 'n')]
    pub without_hash: bool,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "hash")]
/// Replace with a hash.
pub struct HashMask {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: String,
    /// prefix to prepend to the hash
    #[argh(option, short = 'p')]
    pub prefix: Option<String>,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "mask")]
/// Format and mask specific digits.
pub struct FormatMask {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: String,
    /// formatting string, default empty (fallback to original or E164)
    #[argh(option, short = 'f')]
    pub format: Option<String>,

    /// character used for masking (default '*')
    #[argh(option, short = 'm', default = "'*'")]
    pub mask_char: char,

    /// minimum number of digits to mask
    #[argh(option, short = 'c', default = "4")]
    pub min_masked: usize,

    /// maximum number of digits to leave unmasked at the end
    #[argh(option, short = 'u', default = "4")]
    pub max_unmasked: usize,
}

fn load_custom_metadata(metadata_path: Option<&str>) -> Result<PhoneNumberUtil, Box<dyn Error>> {
    let util = if let Some(path) = metadata_path {
        let source: Source = path.parse()?;

        let mut buf = Vec::new();
        source.read()?.read_to_end(&mut buf)?;
        let metadata = PhoneMetadataCollection::decode(buf.as_ref())?;
        PhoneNumberUtil::new_for_metadata(metadata)?
    } else {
        PhoneNumberUtil::new()?
    };
    Ok(util)
}

fn parse_region(region_arg: Option<&str>) -> Result<Option<Region>, Box<dyn Error>> {
    if let Some(r) = region_arg {
        Ok(Some(Region::from_code(r)?))
    } else {
        Ok(None)
    }
}

fn get_hmac_key() -> Vec<u8> {
    match std::env::var("PHONE_HMAC_KEY") {
        Ok(key) => key.into_bytes(),
        Err(_) => {
            eprintln!(
                "[WARN] PHONE_HMAC_KEY env variable is not set. Generating a random key for this session. Hashes will not be deterministic across different runs!"
            );
            let mut key = vec![0u8; 32];
            rand::rng().fill_bytes(&mut key);
            key
        }
    }
}

pub fn execute(command: NumberCommand) -> Result<(), Box<dyn Error>> {
    let util = load_custom_metadata(command.metadata.as_deref())?;
    let util = Arc::new(util);
    let region = parse_region(command.region.as_deref())?;

    match command.action {
        NumberAction::Parse(cmd) => execute_parse(cmd, &util, region),
        NumberAction::Find(cmd) => execute_find(cmd, &util, region),
        NumberAction::Mask(cmd) => execute_mask(cmd, util, region),
    }
}

#[allow(clippy::too_many_arguments)]
fn print_formatted_output(
    stdout: &mut dyn Write,
    util: &PhoneNumberUtil,
    phone_number: &PhoneNumber,
    raw_number: Option<&str>,
    fmt: PhoneNumberFormat,
    format_used_str: &str,
    region: Option<Region>,
    output_mode: &str,
) {
    let formatted_number = util.format(phone_number, fmt);
    let is_valid = phone_number.is_valid();

    match output_mode {
        "plaintext" => {
            cli_write_str!(stdout, &formatted_number);
            cli_write_str!(stdout, "\n");
        }
        "json" => {
            let mut info = json!({
                "formatted_number": formatted_number.as_ref(),
                "format_used": format_used_str,
                "country_code": phone_number.country_code,
                "national_number": phone_number.national_number,
                "region_code": region.map(|r| r.as_region_str()).as_deref(),
                "is_valid": is_valid,
            });
            if let Some(raw) = raw_number {
                info.as_object_mut()
                    .unwrap()
                    .insert("raw_number".to_string(), json!(raw));
            }
            serde_json::to_writer(&mut *stdout, &info).unwrap_or_else(|err| {
                match err.io_error_kind() {
                    Some(io::ErrorKind::BrokenPipe) => std::process::exit(0),
                    _ => panic!("{}", err),
                }
            });
            cli_write_str!(stdout, "\n");
        }
        "wide" => {
            cli_writeln!(stdout, "{:<20} | Value", "Property");
            cli_writeln!(stdout, "{:-<20}-+-{:-<30}", "", "");
            if let Some(raw) = raw_number {
                cli_writeln!(stdout, "{:<20} | {}", "Raw Number", raw);
            }
            cli_writeln!(stdout, "{:<20} | {}", "Formatted Number", formatted_number);
            cli_writeln!(stdout, "{:<20} | {}", "Format Used", format_used_str);
            cli_writeln!(
                stdout,
                "{:<20} | {}",
                "Country Code",
                phone_number.country_code
            );
            cli_writeln!(
                stdout,
                "{:<20} | {}",
                "National Number",
                phone_number.national_number
            );
            cli_writeln!(
                stdout,
                "{:<20} | {}",
                "Region Code",
                region
                    .map(|r| r.as_region_str())
                    .as_deref()
                    .unwrap_or("None")
            );
            cli_writeln!(stdout, "{:<20} | {}", "Is Valid", is_valid);
            cli_write_str!(stdout, "\n");
        }
        _ => {
            cli_writeln!(
                stdout,
                "Unknown output mode: '{}'. Use 'plaintext', 'json', or 'wide'.",
                output_mode
            );
        }
    }
}

fn execute_parse(
    options: ParseCommand,
    util: &PhoneNumberUtil,
    region: Option<Region>,
) -> Result<(), Box<dyn Error>> {
    let phone_number = util.parse(&options.number, region)?;

    let stdout_raw = io::stdout();
    let mut stdout = BufWriter::with_capacity(BUF_CAPACITY, stdout_raw.lock());
    let format_fmt =
        PhoneNumberFormat::from_str(&options.format).unwrap_or(PhoneNumberFormat::E164);

    print_formatted_output(
        &mut stdout,
        util,
        &phone_number,
        Some(&options.number),
        format_fmt,
        &options.format,
        region,
        &options.output,
    );

    stdout.flush()?;
    Ok(())
}

fn execute_find(
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
            FoundToken::Phone(found, _) => {
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

fn execute_mask(
    options: MaskCommand,
    util: Arc<PhoneNumberUtil>,
    region: Option<Region>,
) -> Result<(), Box<dyn Error>> {
    let mask_util = PhoneMaskUtil::new_for_util(util.clone());

    let stdout_raw = io::stdout();
    let mut stdout = BufWriter::with_capacity(BUF_CAPACITY, stdout_raw.lock());
    let source: Source = options.mask_type.input_str().parse()?;
    let factory = PhoneNumberMatcherFactory::new_for_util(util.clone());
    let base_hmac = match &options.mask_type {
        MaskType::Hash(_)
        | MaskType::Token(TokenMask {
            without_hash: false,
            ..
        }) => Some(
            Hmac::<Sha256>::new_from_slice(&get_hmac_key()).expect("HMAC initialization failed"),
        ),
        _ => None,
    };

    let constant_mask_value = if let MaskType::Constant(ref c) = options.mask_type {
        Some(c.value.as_bytes())
    } else {
        None
    };

    let hash_prefix = if let MaskType::Hash(ref h) = options.mask_type {
        h.prefix.as_deref().map(|s| s.as_bytes())
    } else {
        None
    };

    let mask_format_fmt = if let MaskType::Mask(ref m) = options.mask_type {
        m.format.as_ref().map(|format_str| {
            PhoneNumberFormat::from_str(format_str).unwrap_or(PhoneNumberFormat::E164)
        })
    } else {
        None
    };

    let mask_config = if let MaskType::Mask(ref m) = options.mask_type {
        Some(MaskDigitsConfig {
            mask_char: m.mask_char,
            min_masked: m.min_masked,
            max_unmasked: m.max_unmasked,
        })
    } else {
        None
    };

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
            FoundToken::Phone(found, s) => match &options.mask_type {
                MaskType::Constant(_) => {
                    handle_pipe!(stdout.write_all(constant_mask_value.unwrap()));
                }
                MaskType::Token(_) => {
                    if let Some(base) = base_hmac.clone() {
                        let hasher = PhoneMacHasher(base);
                        handle_pipe!(mask_util.tokenize(&found.number, hasher, &mut stdout));
                    } else {
                        handle_pipe!(mask_util.tokenize(&found.number, (), &mut stdout));
                    }
                }
                MaskType::Hash(_) => {
                    let hasher = PhoneMacHasher(base_hmac.clone().unwrap());
                    let hash_val = hasher.hash_phone(&found.number).unwrap();
                    let mut buf = [0; 128];
                    if let Some(p) = hash_prefix {
                        handle_pipe!(stdout.write_all(p));
                        cli_write_str!(stdout, hash_val.as_hex(&mut buf));
                    } else {
                        cli_write_str!(stdout, hash_val.as_hex(&mut buf));
                    }
                }
                MaskType::Mask(_) => {
                    let config = mask_config.unwrap();

                    if let Some(fmt) = mask_format_fmt {
                        let masked = mask_util.format_and_mask(&found.number, fmt, config);
                        cli_write_str!(stdout, &masked);
                    } else {
                        handle_pipe!(mask_util.mask_digits(s, config, &mut stdout));
                    }
                }
            },
            FoundToken::NoPhone(text) => {
                cli_write_str!(stdout, text);
            }
        },
    )?;

    stdout.flush()?;
    Ok(())
}
