use argh::FromArgs;

pub mod metadata;
pub mod number;

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum CommandEnum {
    Metadata(metadata::MetadataCommand),
    Number(number::NumberCommand),
}
