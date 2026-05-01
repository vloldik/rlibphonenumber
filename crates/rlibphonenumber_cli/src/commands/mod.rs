use argh::FromArgs;

pub mod build_metadata;
pub mod find_numbers;
pub mod phone_info;
pub mod validate_metadata;

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum CommandEnum {
    ValidateMetadata(validate_metadata::ValidateMetadataCommand),
    FindNumbers(find_numbers::FindNumbersCommand),
    BuildMetadata(build_metadata::BuildMetadataCommand),
    PhoneInfo(phone_info::PhoneInfoCommand),
}
