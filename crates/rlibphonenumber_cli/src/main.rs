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
        CommandEnum::Metadata(cmd) => {
            commands::metadata::execute(cmd)?;
        }
        CommandEnum::Number(cmd) => commands::number::execute(cmd)?,
    }

    Ok(())
}
