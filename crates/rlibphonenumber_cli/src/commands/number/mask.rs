use std::{
    error::Error,
    io::{self, BufWriter, Write},
    str::FromStr,
    sync::Arc,
};

use argh::FromArgs;
use hmac::{Hmac, KeyInit};
use rand::Rng;
use sha2::Sha256;

use rlibphonenumber::{
    PhoneNumberFormat, PhoneNumberUtil,
    enums::Region,
    interfaces::PhoneHasher,
    phonenumber_mask::{MaskDigitsConfig, PhoneMacHasher, PhoneMaskUtil},
    phonenumber_matcher::{Leniency, PhoneNumberMatcherFactory},
};

use crate::sources::{FoundToken, SearchNumbers, Source};

use super::{BUF_CAPACITY, cli_write_str, handle_pipe, parse_leniency};

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
        .as_deref()
        .unwrap_or("-")
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "constant")]
/// Replace with a constant string.
pub struct ConstantMask {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: Option<String>,
    /// the constant string to replace the phone number with
    #[argh(option, default = "String::from(\"<REDACTED>\")")]
    pub value: String,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "token")]
/// Replace with a semantic token.
pub struct TokenMask {
    /// input source (file, URL, ssh, git)
    #[argh(positional)]
    pub input: Option<String>,
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
    pub input: Option<String>,
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
    pub input: Option<String>,
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

pub fn execute(
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
            FoundToken::Phone(found) => match &options.mask_type {
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
                    let masked =
                        mask_util.format_and_mask(&found.number, mask_format_fmt.unwrap(), config);
                    cli_write_str!(stdout, &masked);
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
