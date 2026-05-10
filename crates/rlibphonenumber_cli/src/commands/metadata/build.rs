use std::{fs, path::PathBuf};

use prost::Message;
use rlibphonenumber::metadata_validator::validate_metadata;

use crate::parser::{builder::MetadataBuilder, transform_for_rust::transform_for_rust};

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "build")]
/// Build binary (and optionally Rust module) metadata files from an XML source.
pub struct BuildAction {
    /// output directory where the binary and Rust files will be written
    #[argh(positional)]
    pub output_dir: String,

    /// base name for generated files (e.g. core, lite_core, test_core)
    #[argh(positional)]
    pub basename: String,

    /// name of the exported Rust constant for the metadata
    #[argh(option, default = "String::from(\"METADATA\")")]
    pub const_name: String,

    /// skip metadata validation after building
    #[argh(switch)]
    pub skip_validate: bool,

    /// also emit a .rs module with an include_bytes! constant
    #[argh(switch, short = 'm')]
    pub generate_mod: bool,
}

pub fn execute(
    input: String,
    filter: Option<String>,
    alternate_formats: bool,
    action: BuildAction,
) -> Result<(), Box<dyn std::error::Error>> {
    let builder = MetadataBuilder::new(filter);

    let collection = builder.build_from_source(
        input.parse()?,
        /* short numbers not supported yet */ false,
        alternate_formats,
    )?;
    let transformed = transform_for_rust(collection);

    if !action.skip_validate {
        validate_metadata(transformed.clone(), alternate_formats)?;
    }

    let bytes = transformed.encode_to_vec();

    let bin_filename = format!("{}.bin", action.basename);
    let bin_path = PathBuf::from(&action.output_dir).join(&bin_filename);
    fs::write(&bin_path, &bytes)?;

    if action.generate_mod {
        let rs_filename = format!("{}.rs", action.basename);
        let rs_path = PathBuf::from(&action.output_dir).join(&rs_filename);
        let rs_code = format!(
            "// This file is auto-generated. Do not edit.\n\
             pub static {}: &[u8] = include_bytes!(\"{}\");\n",
            action.const_name, bin_filename
        );
        fs::write(&rs_path, rs_code)?;
    }

    println!("Success! Metadata written to {}", action.output_dir);
    Ok(())
}
