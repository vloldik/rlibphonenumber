use argh::FromArgs;
use prost::Message;
use std::fs;
use std::path::PathBuf;

use rlibphonenumber::metadata_validator::validate_metadata;

use crate::parser::{builder::MetadataBuilder, transform_for_rust::transform_for_rust};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "build-metadata")]
/// Rust metadata generator for rlibphonenumbers.
pub struct BuildMetadataCommand {
    /// input source. Supports:
    /// - Local file paths (e.g., path/to/file.xml)
    /// - HTTP/HTTPS URLs (e.g., https://example.com/data.bin)
    /// - SSH paths (e.g., user@host:/path/to/file)
    /// - Git repositories (e.g., git://github.com/user/repo.git?file=data.xml&branch=main)
    #[argh(positional)]
    pub input_xml: String,

    /// output directory path where the binary and rust files will be saved
    #[argh(positional)]
    pub output_dir: String,

    /// base name for the generated files (e.g.: core, lite_core, test_core)
    #[argh(positional)]
    pub basename: String,

    /// name of the exported rust constant for the metadata
    #[argh(option, default = "String::from(\"METADATA\")")]
    pub const_name: String,

    /// skip metadata validation tests
    #[argh(switch)]
    pub skip_validate: bool,

    /// validate as alternate formats
    #[argh(switch)]
    pub alternate_formats: bool,

    /// generate rust module
    #[argh(switch, short = 'm')]
    pub generate_mod: bool,

    /// custom CEL expression to filter out metadata fields.
    #[argh(option)]
    pub filter: Option<String>,
}

pub fn execute(options: BuildMetadataCommand) -> Result<(), Box<dyn std::error::Error>> {
    let builder = MetadataBuilder::new(options.filter);

    let collection = builder.build_from_source(
        options.input_xml.parse()?,
        /* short number not supported yet */ false,
        options.alternate_formats,
    )?;
    let transformed = transform_for_rust(collection);

    if !options.skip_validate {
        validate_metadata(transformed.clone(), options.alternate_formats)?;
    }

    let bytes = transformed.encode_to_vec();

    let bin_filename = format!("{}.bin", options.basename);
    let bin_path = PathBuf::from(&options.output_dir).join(&bin_filename);
    fs::write(&bin_path, &bytes)?;

    if options.generate_mod {
        let rs_filename = format!("{}.rs", options.basename);
        let rs_path = PathBuf::from(&options.output_dir).join(&rs_filename);

        let rs_code = format!(
            "// This file is auto-generated. Do not edit.\n\
        pub static {}: &[u8] = include_bytes!(\"{}\");\n",
            options.const_name, bin_filename
        );
        fs::write(&rs_path, rs_code)?;
    }

    println!("Success! Metadata written to {}", options.output_dir);

    Ok(())
}
