use argh::FromArgs;

mod build;
mod validate;

use build::BuildAction;
use validate::ValidateAction;

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "metadata")]
/// Build or validate phone number metadata from an XML or Binary source.
pub struct MetadataCommand {
    /// input source. Supports:
    /// - Local file paths (e.g., path/to/file.xml)
    /// - HTTP/HTTPS URLs (e.g., https://example.com/data.bin)
    /// - SSH paths (e.g., user@host:/path/to/file)
    /// - Git repositories (e.g., git://github.com/user/repo.git?file=data.xml&branch=main)
    #[argh(option, short = 'i')]
    pub input: String,

    /// custom CEL expression to filter metadata fields (XML sources only)
    #[argh(option, short = 'f')]
    pub filter: Option<String>,

    /// treat input as alternate formats, relaxing common metadata rules
    #[argh(switch)]
    pub alternate_formats: bool,

    #[argh(subcommand)]
    pub action: MetadataAction,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum MetadataAction {
    Build(BuildAction),
    Validate(ValidateAction),
}

pub fn execute(cmd: MetadataCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd.action {
        MetadataAction::Build(action) => {
            build::execute(cmd.input, cmd.filter, cmd.alternate_formats, action)
        }
        MetadataAction::Validate(_) => {
            validate::execute(cmd.input, cmd.filter, cmd.alternate_formats)
        }
    }
}
