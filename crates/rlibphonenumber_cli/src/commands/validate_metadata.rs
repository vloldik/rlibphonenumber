use std::{error::Error, fs, path::Path};

use argh::FromArgs;
use prost::Message;
use rlibphonenumber::{PhoneMetadataCollection, metadata_validator::validate_metadata};

use crate::parser::{builder::MetadataBuilder, transform_for_rust::transform_for_rust};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "validate-metadata")]
/// Tests and validates phone number metadata from an XML or Binary file.
pub struct ValidateMetadataCommand {
    /// input file path (can be .xml or .bin)
    #[argh(positional)]
    pub input_file: String,

    /// validate input ignoring common metadata rules, as for alternate formats
    #[argh(switch)]
    pub alternate_formats: bool,

    /// custom CEL expression to filter out metadata fields (used only if input is XML)
    #[argh(option)]
    pub filter: Option<String>,
}

/// CLI execution entry point: Handles file reading, parsing, and outputting results.
pub fn execute(options: ValidateMetadataCommand) -> Result<(), Box<dyn Error>> {
    let ext = Path::new(&options.input_file)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let collection = if ext.eq_ignore_ascii_case("xml") {
        let builder = MetadataBuilder::new(options.filter);
        transform_for_rust(builder.build_from_file(&options.input_file)?)
    } else if ext.eq_ignore_ascii_case("bin") {
        let bytes = fs::read(&options.input_file)?;
        PhoneMetadataCollection::decode(bytes.as_slice())?
    } else {
        return Err("Input file must have a .xml or .bin extension".into());
    };

    println!(
        "Loaded metadata for {} regions. Starting validation...",
        collection.metadata.len()
    );

    // Call the pure validation function
    validate_metadata(collection, options.alternate_formats)?;

    println!("Success! All metadata validations passed.");
    Ok(())
}
