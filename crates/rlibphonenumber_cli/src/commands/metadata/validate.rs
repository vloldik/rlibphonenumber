use std::{io::Read, path::Path};

use prost::Message;
use rlibphonenumber::{PhoneMetadataCollection, metadata_validator::validate_metadata};

use crate::{
    parser::{builder::MetadataBuilder, transform_for_rust::transform_for_rust},
    sources::{ReadSource, Source},
};

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "validate")]
/// Validate phone number metadata (no output files are written).
pub struct ValidateAction {}

pub fn execute(
    input: String,
    filter: Option<String>,
    alternate_formats: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let source: Source = input.parse()?;
    let filename = source.file_name().unwrap_or(input.clone());
    let ext = Path::new(&filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let collection = if ext.eq_ignore_ascii_case("xml") {
        let builder = MetadataBuilder::new(filter);
        transform_for_rust(builder.build_from_source(
            source,
            /* short numbers not supported yet */ false,
            alternate_formats,
        )?)
    } else if ext.eq_ignore_ascii_case("bin") {
        let mut buf = Vec::new();
        source.read()?.read_to_end(&mut buf)?;
        PhoneMetadataCollection::decode(buf.as_ref())?
    } else {
        return Err("Input file must have a .xml or .bin extension".into());
    };

    println!(
        "Loaded metadata for {} regions. Starting validation...",
        collection.metadata.len()
    );

    validate_metadata(collection, alternate_formats)?;

    println!("Success! All metadata validations passed.");
    Ok(())
}
