use std::error::Error;

use argh::FromArgs;

use crate::commands::CommandEnum;

mod commands;
mod parser;
mod sources;

#[derive(FromArgs, Debug)]
/// rlibphonenumber CLI utility
struct Cli {
    #[argh(subcommand)]
    command: CommandEnum,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli: Cli = argh::from_env();

    match cli.command {
        CommandEnum::BuildMetadata(cmd) => {
            commands::build_metadata::execute(cmd)?;
        }
        CommandEnum::ValidateMetadata(cmd) => {
            commands::validate_metadata::execute(cmd)?;
        }
        CommandEnum::Number(cmd) => commands::number_command::execute(cmd)?,
    }

    Ok(())
}
