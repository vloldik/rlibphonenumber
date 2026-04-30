use std::error::Error;

use argh::FromArgs;

use crate::commands::CommandEnum;

mod commands;
mod parser;

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
        CommandEnum::PhoneInfo(cmd) => {
            commands::phone_info::execute(cmd)?;
        }
        CommandEnum::ValidateMetadata(cmd) => {
            commands::validate_metadata::execute(cmd)?;
        }
    }

    Ok(())
}
