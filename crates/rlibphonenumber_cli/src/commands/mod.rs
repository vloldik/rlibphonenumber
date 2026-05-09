use argh::FromArgs;

pub mod build_metadata;
pub mod number_command;
pub mod validate_metadata;

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum CommandEnum {
    ValidateMetadata(validate_metadata::ValidateMetadataCommand),
    Number(number_command::NumberCommand),
    BuildMetadata(build_metadata::BuildMetadataCommand),
}
