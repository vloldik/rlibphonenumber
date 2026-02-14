use rlibphonenumber::{
    PhoneNumber,
    PhoneNumberFormat,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let number_string = "+1-587-530-2271";

    // 1. Parse the number
    // You can use the standard FromStr trait:
    let number: PhoneNumber = number_string.parse()?;
    
    // Or explicitly via the utility:
    // let number = PHONE_NUMBER_UTIL.parse(number_string)?;

    println!("✅ Successfully parsed number.");
    println!("   - Country Code: {}", number.country_code());
    println!("   - National Number: {}", number.national_number());

    // 2. Validate the number
    // `is_valid()` performs a full validation (length, prefix, region rules)
    let is_valid = number.is_valid();
    println!(
        "\nIs the number valid? {}",
        if is_valid { "Yes" } else { "No" }
    );

    if !is_valid {
        return Ok(());
    }

    // 3. Format the number
    // Display trait uses E164 by default
    println!("\nDefault Display: {}", number); 

    let e164_format = number.format_as(PhoneNumberFormat::E164);
    let international_format = number.format_as(PhoneNumberFormat::International);
    let national_format = number.format_as(PhoneNumberFormat::National);
    let rfc3966_format = number.format_as(PhoneNumberFormat::RFC3966);

    println!("Formatted Outputs:");
    println!("   - E.164:         {}", e164_format);         // +15875302271
    println!("   - International: {}", international_format); // +1 587-530-2271
    println!("   - National:      {}", national_format);      // (587) 530-2271
    println!("   - RFC3966:       {}", rfc3966_format);       // tel:+1-587-530-2271

    // 4. Get additional information
    let number_type = number.get_type(); // e.g., Mobile, FixedLine
    let region_code = number.get_region_code(); // e.g., "CA"

    println!("\nInfo:");
    println!("   - Type:   {:?}", number_type);
    println!("   - Region: {:?}", region_code.unwrap_or("Unknown"));

    Ok(())
}
