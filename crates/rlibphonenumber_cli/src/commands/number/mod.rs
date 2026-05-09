use std::{
    error::Error,
    io::{self, Read},
    str::FromStr,
    sync::Arc,
};

use argh::FromArgs;
use prost::Message;
use rlibphonenumber::{
    PhoneMetadataCollection, PhoneNumberUtil, enums::Region, phonenumber_matcher::Leniency,
};

use crate::sources::{ReadSource, Source};

pub mod find;
pub mod mask;
pub mod parse;

use find::FindCommand;
use mask::MaskCommand;
use parse::ParseCommand;

pub(super) const BUF_CAPACITY: usize = 256 * 1024;

#[cold]
#[inline(never)]
pub(super) fn handle_io_err(e: io::Error) {
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
            Err(e) => $crate::commands::number::handle_io_err(e),
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

pub(super) use cli_write_str;
pub(super) use cli_writeln;
pub(super) use handle_pipe;

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

pub(super) fn load_custom_metadata(
    metadata_path: Option<&str>,
) -> Result<PhoneNumberUtil, Box<dyn Error>> {
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

pub(super) fn parse_region(region_arg: Option<&str>) -> Result<Option<Region>, Box<dyn Error>> {
    if let Some(r) = region_arg {
        Ok(Some(Region::from_code(r)?))
    } else {
        Ok(None)
    }
}

pub fn execute(command: NumberCommand) -> Result<(), Box<dyn Error>> {
    let util = load_custom_metadata(command.metadata.as_deref())?;
    let util = Arc::new(util);
    let region = parse_region(command.region.as_deref())?;

    match command.action {
        NumberAction::Parse(cmd) => parse::execute(cmd, &util, region),
        NumberAction::Find(cmd) => find::execute(cmd, &util, region),
        NumberAction::Mask(cmd) => mask::execute(cmd, util, region),
    }
}

pub fn parse_leniency(s: &str) -> Result<Leniency, String> {
    Leniency::from_str(s).map_err(|err| err.to_string())
}
