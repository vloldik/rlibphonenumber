use argh::FromArgs;

pub mod build_metadata;
pub mod phone_info;

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum CommandEnum {
    BuildMetadata(build_metadata::BuildMetadataCommand),
    PhoneInfo(phone_info::PhoneInfoCommand),
}
