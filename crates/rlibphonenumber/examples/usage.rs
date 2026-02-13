use rlibphonenumber::{
    // or instead you can use PhoneNumberUtil::new()
    PHONE_NUMBER_UTIL,
    PhoneNumberFormat,
    PhoneNumberStaticExt,
    Region,
};

fn main() {
    let number_string = "+1-587-530-2271";

    // 1. Parse the number
    match PHONE_NUMBER_UTIL.parse_with_default_region(number_string, Region::US) {
        Ok(number) => {
            println!("✅ Successfully parsed number.");
            println!(
                "   - Original input: '{}' (in '{}')",
                number_string,
                Region::US.as_ref()
            );
            println!("   - Country Code: {}", number.country_code());
            println!("   - National Number: {}", number.national_number());

            // 2. Validate the number
            // `is_valid_number` performs a full validation, checking length,
            // prefix, and other region-specific rules.
            let is_valid = number.is_valid();
            println!(
                "\nIs the number valid? {}",
                if is_valid { "Yes" } else { "No" }
            );

            if !is_valid {
                return;
            }

            // 3. Format the number in different standard formats
            let international_format =
                PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::International);
            let national_format = PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::National);
            let e164_format = number.format(PhoneNumberFormat::E164);
            let rfc3966_format = number.format(PhoneNumberFormat::RFC3966);

            println!("\nFormatted Outputs:");
            println!("   - International: {}", international_format);
            println!("   - National:      {}", national_format);
            println!("   - E.164:         {}", e164_format);
            println!("   - RFC3966:       {}", rfc3966_format);

            // 4. Get additional information about the number
            let number_type = number.get_type();
            let number_region = PHONE_NUMBER_UTIL.get_region_code_for_number(&number);

            println!("\nAdditional Information:");
            println!("   - Number Type:   {:?}", number_type); // e.g., FixedLine
            println!("   - Number Region: {}", number_region); // e.g., US
        }
        Err(e) => {
            // Handle parsing errors, e.g., if the number is invalid or not a number.
            println!("❌ Error parsing number: {:?}", e);
        }
    }
}
