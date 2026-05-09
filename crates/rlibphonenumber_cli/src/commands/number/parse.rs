use std::{
    error::Error,
    io::{self, BufWriter, Write},
    str::FromStr,
};

use argh::FromArgs;
use serde_json::json;

use rlibphonenumber::{PhoneNumber, PhoneNumberFormat, PhoneNumberUtil, enums::Region};

use super::{BUF_CAPACITY, cli_write_str, cli_writeln, handle_pipe};

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "parse")]
/// Get parsed information about a specific phone number.
pub struct ParseCommand {
    /// the phone number to parse (e.g. +1234567890)
    #[argh(positional)]
    pub number: String,

    /// output mode: 'plaintext', 'json', or 'wide'
    #[argh(option, short = 'o', default = "String::from(\"plaintext\")")]
    pub output: String,

    /// phone number format: 'e164', 'international', 'national', 'rfc3966'
    #[argh(option, short = 'f', default = "String::from(\"e164\")")]
    pub format: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn print_formatted_output(
    stdout: &mut dyn Write,
    util: &PhoneNumberUtil,
    phone_number: &PhoneNumber,
    raw_number: Option<&str>,
    fmt: PhoneNumberFormat,
    format_used_str: &str,
    region: Option<Region>,
    output_mode: &str,
) {
    let formatted_number = util.format(phone_number, fmt);
    let is_valid = phone_number.is_valid();

    match output_mode {
        "plaintext" => {
            cli_write_str!(stdout, &formatted_number);
            cli_write_str!(stdout, "\n");
        }
        "json" => {
            let mut info = json!({
                "formatted_number": formatted_number.as_ref(),
                "format_used": format_used_str,
                "country_code": phone_number.country_code,
                "national_number": phone_number.national_number,
                "region_code": region.map(|r| r.as_region_str()).as_deref(),
                "is_valid": is_valid,
            });
            if let Some(raw) = raw_number {
                info.as_object_mut()
                    .unwrap()
                    .insert("raw_number".to_string(), json!(raw));
            }
            serde_json::to_writer(&mut *stdout, &info).unwrap_or_else(|err| {
                match err.io_error_kind() {
                    Some(io::ErrorKind::BrokenPipe) => std::process::exit(0),
                    _ => panic!("{}", err),
                }
            });
            cli_write_str!(stdout, "\n");
        }
        "wide" => {
            cli_writeln!(stdout, "{:<20} | Value", "Property");
            cli_writeln!(stdout, "{:-<20}-+-{:-<30}", "", "");
            if let Some(raw) = raw_number {
                cli_writeln!(stdout, "{:<20} | {}", "Raw Number", raw);
            }
            cli_writeln!(stdout, "{:<20} | {}", "Formatted Number", formatted_number);
            cli_writeln!(stdout, "{:<20} | {}", "Format Used", format_used_str);
            cli_writeln!(
                stdout,
                "{:<20} | {}",
                "Country Code",
                phone_number.country_code
            );
            cli_writeln!(
                stdout,
                "{:<20} | {}",
                "National Number",
                phone_number.national_number
            );
            cli_writeln!(
                stdout,
                "{:<20} | {}",
                "Region Code",
                region
                    .map(|r| r.as_region_str())
                    .as_deref()
                    .unwrap_or("None")
            );
            cli_writeln!(stdout, "{:<20} | {}", "Is Valid", is_valid);
            cli_write_str!(stdout, "\n");
        }
        _ => {
            cli_writeln!(
                stdout,
                "Unknown output mode: '{}'. Use 'plaintext', 'json', or 'wide'.",
                output_mode
            );
        }
    }
}

pub fn execute(
    options: ParseCommand,
    util: &PhoneNumberUtil,
    region: Option<Region>,
) -> Result<(), Box<dyn Error>> {
    let phone_number = util.parse(&options.number, region)?;

    let stdout_raw = io::stdout();
    let mut stdout = BufWriter::with_capacity(BUF_CAPACITY, stdout_raw.lock());
    let format_fmt =
        PhoneNumberFormat::from_str(&options.format).unwrap_or(PhoneNumberFormat::E164);

    print_formatted_output(
        &mut stdout,
        util,
        &phone_number,
        Some(&options.number),
        format_fmt,
        &options.format,
        region,
        &options.output,
    );

    stdout.flush()?;
    Ok(())
}
