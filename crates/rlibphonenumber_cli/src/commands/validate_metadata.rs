use std::{error::Error, io::Read, path::Path};

use argh::FromArgs;
use prost::Message;
use rlibphonenumber::{PhoneMetadataCollection, metadata_validator::validate_metadata};

use crate::{
    parser::{builder::MetadataBuilder, transform_for_rust::transform_for_rust},
    sources::Source,
};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "validate-metadata")]
/// Tests and validates phone number metadata from an XML or Binary file.
pub struct ValidateMetadataCommand {
    /// input source. Supports:
    /// - Local file paths (e.g., path/to/file.xml)
    /// - HTTP/HTTPS URLs (e.g., https://example.com/data.bin)
    /// - SSH paths (e.g., user@host:/path/to/file)
    /// - Git repositories (e.g., git://github.com/user/repo.git?file=data.xml&branch=main)
    #[argh(positional)]
    pub input: String,

    /// validate input ignoring common metadata rules, as for alternate formats
    #[argh(switch)]
    pub alternate_formats: bool,

    /// custom CEL expression to filter out metadata fields (used only if input is XML)
    #[argh(option)]
    pub filter: Option<String>,
}

/// CLI execution entry point: Handles file reading, parsing, and outputting results.
pub fn execute(options: ValidateMetadataCommand) -> Result<(), Box<dyn Error>> {
    let source: Source = options.input.parse()?;
    let filename = source.file_name().unwrap_or(options.input.clone());
    let ext = Path::new(&filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let collection = if ext.eq_ignore_ascii_case("xml") {
        let builder = MetadataBuilder::new(options.filter);
        transform_for_rust(builder.build_from_source(
            source,
            /* short numbers not supported yet */ false,
            options.alternate_formats,
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

    // Call the pure validation function
    validate_metadata(collection, options.alternate_formats)?;

    println!("Success! All metadata validations passed.");
    Ok(())
}
