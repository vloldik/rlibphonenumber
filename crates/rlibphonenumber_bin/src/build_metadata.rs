use argh::FromArgs;
use std::fs;
use std::path::PathBuf;

use crate::parser::builder::MetadataBuilder;
use rlibphonenumber::{NumberFormat, PhoneMetadataCollection, PhoneNumberDesc};

#[derive(FromArgs, Debug)]
/// Rust metadata generator for rlibphonenumbers.
///
/// This utility parses phone number metadata from XML, applies an optional CEL
/// (Common Expression Language) filter to reduce the dataset size, transforms
/// regular expressions, and serializes the result into a binary format using `rkyv`.
pub struct Options {
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

    /// custom CEL (Common Expression Language) expression to filter out metadata fields.
    /// The expression should evaluate to `true` for fields that MUST BE DROPPED.
    ///
    /// Available CEL context variables:
    /// - `field` (string): The current field name (e.g., "example_number", "national_prefix").
    /// - `parent` (string): The parent struct name (e.g., "fixed_line", "mobile"). Empty `""` if root.
    /// - `region` (string): The region ID (e.g., "US", "RU").
    /// - `country_code` (int): The country calling code.
    /// - `is_main_country` (bool): True if this region is the main one for its calling code.
    ///
    /// Example usage:
    /// --filter "region == 'US' && field == 'example_number'"
    /// --filter "parent != '' && parent != 'mobile'"
    #[argh(option)]
    pub filter: Option<String>,
}

pub fn build_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let options: Options = argh::from_env();
    let builder = MetadataBuilder::new(options.filter);

    let collection = builder.build_from_file(&options.input_xml)?;
    let transformed = transform_for_rust(collection);

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&transformed)?;

    let bin_filename = format!("{}.bin", options.basename);
    let bin_path = PathBuf::from(&options.output_dir).join(&bin_filename);
    fs::write(&bin_path, &bytes)?;

    let rs_filename = format!("{}.rs", options.basename);
    let rs_path = PathBuf::from(&options.output_dir).join(&rs_filename);

    let rs_code = format!(
        "// This file is auto-generated. Do not edit.\n\
         #[repr(C, align(16))]\n\
         struct AlignedBytes<const N: usize>(pub [u8; N]);\n\
         static METADATA_BYTES: AlignedBytes<{{ include_bytes!(\"{}\").len() }}> = AlignedBytes(*include_bytes!(\"{}\"));\n\
         pub const {}: &[u8] = &METADATA_BYTES.0;\n",
        bin_filename, bin_filename, options.const_name
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
