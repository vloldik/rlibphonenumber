use argh::FromArgs;
use prost::Message;
use std::fs;
use std::path::PathBuf;

use crate::parser::builder::MetadataBuilder; // Убедитесь, что этот путь актуален для вашего проекта
use rlibphonenumber::{NumberFormat, PhoneMetadataCollection, PhoneNumberDesc};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "build-metadata")]
/// Rust metadata generator for rlibphonenumbers.
pub struct BuildMetadataCommand {
    /// input xml file path (e.g., PhoneNumberMetadata.xml)
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

    /// custom CEL expression to filter out metadata fields.
    #[argh(option)]
    pub filter: Option<String>,
}

pub fn execute(options: BuildMetadataCommand) -> Result<(), Box<dyn std::error::Error>> {
    let builder = MetadataBuilder::new(options.filter);

    let collection = builder.build_from_file(&options.input_xml)?;
    let transformed = transform_for_rust(collection);

    let bytes = transformed.encode_to_vec();

    let bin_filename = format!("{}.bin", options.basename);
    let bin_path = PathBuf::from(&options.output_dir).join(&bin_filename);
    fs::write(&bin_path, &bytes)?;

    let rs_filename = format!("{}.rs", options.basename);
    let rs_path = PathBuf::from(&options.output_dir).join(&rs_filename);

    let rs_code = format!(
        "// This file is auto-generated. Do not edit.\n\
        pub static {}: &[u8] = include_bytes!(\"{}\");\n",
        options.const_name, bin_filename
    );
    fs::write(&rs_path, rs_code)?;

    println!(
        "Success! Metadata written to {} and {}",
        bin_path.display(),
        rs_path.display()
    );

    Ok(())
}

fn wrap_regex_opt(pattern: &mut Option<String>) {
    if let Some(p) = pattern
        && !p.is_empty()
    {
        *p = format!("^(?:{})$", p);
    }
}

fn wrap_regex(pattern: &mut String) {
    if !pattern.is_empty() {
        *pattern = format!("^(?:{})$", pattern);
    }
}

#[allow(deprecated)]
fn transform_desc(desc: &mut Option<PhoneNumberDesc>) {
    if let Some(d) = desc {
        wrap_regex_opt(&mut d.national_number_pattern);
    }
}

#[allow(deprecated)]
fn transform_format(format: &mut NumberFormat) {
    wrap_regex(&mut format.pattern);

    for lp in &mut format.leading_digits_pattern {
        wrap_regex(lp);
    }
}

#[allow(deprecated)]
fn transform_for_rust(mut collection: PhoneMetadataCollection) -> PhoneMetadataCollection {
    for meta in &mut collection.metadata {
        wrap_regex_opt(&mut meta.leading_digits);
        wrap_regex_opt(&mut meta.international_prefix);
        wrap_regex_opt(&mut meta.national_prefix_for_parsing);

        transform_desc(&mut meta.general_desc);
        transform_desc(&mut meta.fixed_line);
        transform_desc(&mut meta.mobile);
        transform_desc(&mut meta.toll_free);
        transform_desc(&mut meta.premium_rate);
        transform_desc(&mut meta.shared_cost);
        transform_desc(&mut meta.personal_number);
        transform_desc(&mut meta.voip);
        transform_desc(&mut meta.pager);
        transform_desc(&mut meta.uan);
        transform_desc(&mut meta.emergency);
        transform_desc(&mut meta.voicemail);
        transform_desc(&mut meta.short_code);
        transform_desc(&mut meta.standard_rate);
        transform_desc(&mut meta.carrier_specific);
        transform_desc(&mut meta.sms_services);
        transform_desc(&mut meta.no_international_dialling);

        for fmt in &mut meta.number_format {
            transform_format(fmt);
        }

        for fmt in &mut meta.intl_number_format {
            transform_format(fmt);
        }
    }
    collection
}
