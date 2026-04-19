use std::str::FromStr;

use argh::FromArgs;
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumber, PhoneNumberFormat};
use serde_json::json;

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "phone-info")]
/// Get parsed information about a specific phone number.
pub struct PhoneInfoCommand {
    /// the phone number to parse (e.g. +1234567890)
    #[argh(positional)]
    pub number: String,

    /// output mode: 'plaintext', 'json', or 'wide'
    #[argh(option, default = "String::from(\"plaintext\")")]
    pub output: String,

    /// phone number format: 'e164', 'international', 'national', 'rfc3966'
    #[argh(option, default = "String::from(\"e164\")")]
    pub format: String,

    /// default region for given phone
    #[argh(option)]
    pub region: Option<String>,
}

pub fn execute(options: PhoneInfoCommand) -> Result<(), Box<dyn std::error::Error>> {
    let phone_number = if let Some(region) = &options.region {
        PHONE_NUMBER_UTIL.parse_with_default_region(&options.number, region)
    } else {
        PhoneNumber::from_str(&options.number)
    }?;
    let phone_format = PhoneNumberFormat::from_str(&options.format)?;
    let formatted_number = phone_number.format_as(phone_format);
    let country_code = phone_number.country_code;
    let national_number = phone_number.national_number;
    let is_valid = phone_number.is_valid();
    let region_code = phone_number.get_region_code().unwrap_or_default();

    match options.output.as_str() {
        "plaintext" => {
            println!("{}", formatted_number);
        }
        "json" => {
            let info = json!({
                "raw_number": options.number,
                "formatted_number": formatted_number,
                "format_used": options.format,
                "country_code": country_code,
                "national_number": national_number,
                "region_code": region_code,
                "is_valid": is_valid,
            });
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        "wide" => {
            println!("{:<20} | Value", "Property");
            println!("{:-<20}-+-{:-<30}", "", "");
            println!("{:<20} | {}", "Raw Number", options.number);
            println!("{:<20} | {}", "Formatted Number", formatted_number);
            println!("{:<20} | {}", "Format Used", options.format);
            println!("{:<20} | {}", "Country Code", country_code);
            println!("{:<20} | {}", "National Number", national_number);
            println!("{:<20} | {}", "Region Code", region_code);
            println!("{:<20} | {}", "Is Valid", is_valid);
        }
        _ => {
            eprintln!(
                "Unknown output mode: '{}'. Use 'plaintext', 'json', or 'wide'.",
                options.output
            );
        }
    }

    Ok(())
}
