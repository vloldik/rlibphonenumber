# Rlibphonenumber

[![Crates.io](https://img.shields.io/crates/v/rlibphonenumber.svg)](https://crates.io/crates/rlibphonenumber)
[![Docs.rs](https://docs.rs/phonenumber/badge.svg)](https://docs.rs/rlibphonenumber)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A Rust port of Google's comprehensive library for parsing, formatting, and validating international phone numbers.

**Built on base libphonenumber 9.0.8**
**Used metadata version: 9.0.9**

## Overview

This library is a new adaptation of Google's `libphonenumber` for Rust. Its primary goal is to provide a powerful and efficient tool for handling phone numbers, with a structure that is intuitively close to the original C++ version.

You might be aware of an existing Rust implementation of `libphonenumber`. However, its maintenance has slowed, and I believe that a fresh start is the best path forward. This project aims to deliver a more direct and familiar port for developers acquainted with the C++ or Java versions of the original library.

This library gives you access to a wide range of functionalities, including:
*   Parsing and formatting phone numbers.
*   Validating phone numbers for all regions of the world.
*   Determining the number type (e.g., Mobile, Fixed-line, Toll-free).
*   Providing example numbers for every country.

## Performance

The following benchmarks were run against the `rust-phonenumber` crate. All tests were performed on the same machine and dataset. *Lower is better.*

### Formatting

| Format | rlibphonenumber (this crate) | rust-phonenumber | Performance Gain |
|:---|:---:|:---:|:---:|
| **E164** | **~668 ns** | ~12.82 µs | **~19x faster** |
| **International** | **~11.76 µs** | ~17.20 µs | **~1.5x faster** |
| **National** | **~15.19 µs** | ~22.66 µs | **~1.5x faster** |
| **RFC3966** | **~13.41 µs** | ~18.59 µs | **~1.4x faster** |

### Parsing

| Task | rlibphonenumber (this crate) | rust-phonenumber | Performance Gain |
|:--- |:---:|:---:|:---:|
| **Parse** | **~11.60 µs** | ~13.45 µs | **~16% faster** |

## Installation

Add `rlibphonenumber` to your `Cargo.toml`:

```toml
[dependencies]
rlibphonenumber = "0.2.0" # Please use the latest version from crates.io
```

## Getting Started: A Detailed Example

Using the library is straightforward. The `PhoneNumberUtil` struct is the main entry point for all operations. For convenience, a thread-safe static instance, `PHONE_NUMBER_UTIL`, is provided.

Here is a detailed example that demonstrates how to parse a number, validate it, and format it in several standard ways.

```rust
use rlibphonenumber::{
    // or instead you can use PhoneNumberUtil::new()
    PHONE_NUMBER_UTIL, 
    PhoneNumberFormat, 
};
#[test]
fn main() {
    let number_string = "+1-587-530-2271";
    let region_code = "US"; // United States

    // 1. Parse the number
    match PHONE_NUMBER_UTIL.parse(number_string, region_code) {
        Ok(number) => {
            println!("✅ Successfully parsed number.");
            println!("   - Original input: '{}' (in '{}')", number_string, region_code);
            println!("   - Country Code: {}", number.country_code());
            println!("   - National Number: {}", number.national_number());
            
            // 2. Validate the number
            // `is_valid_number` performs a full validation, checking length,
            // prefix, and other region-specific rules.
            let is_valid = PHONE_NUMBER_UTIL.is_valid_number(&number);
            println!("\nIs the number valid? {}", if is_valid { "Yes" } else { "No" });

            if !is_valid {
                return;
            }

            // 3. Format the number in different standard formats
            let international_format = PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::International);
            let national_format = PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::National);
            let e164_format = PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::E164);
            let rfc3966_format = PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::RFC3966);

            println!("\nFormatted Outputs:");
            println!("   - International: {}", international_format);
            println!("   - National:      {}", national_format);
            println!("   - E.164:         {}", e164_format);
            println!("   - RFC3966:       {}", rfc3966_format);
            
            // 4. Get additional information about the number
            let number_type = PHONE_NUMBER_UTIL.get_number_type(&number);
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
```

### Expected Output:

```text
✅ Successfully parsed number.
   - Original input: '+1-587-530-2271' (in 'US')
   - Country Code: 1
   - National Number: 5875302271

Is the number valid? Yes

Formatted Outputs:
   - International: +1 587-530-2271
   - National:      (587) 530-2271
   - E.164:         +15875302271
   - RFC3966:       tel:+1-587-530-2271

Additional Information:
   - Number Type:   FixedLineOrMobile
   - Number Region: CA
```

## Project Status

The library is under active development. The core `PhoneNumberUtil` is fully implemented and passes the original library's test suite.

The project roadmap includes porting these additional components:

*   **`AsYouTypeFormatter`**: For formatting phone numbers as a user types.
*   **`PhoneNumberOfflineGeocoder`**: To provide geographical location information for a number.
*   **`PhoneNumberToCarrierMapper`**: To identify the carrier associated with a number.

## Contributing

Contributions are highly welcome! Whether you are fixing a bug, improving documentation, or helping to port a new module, your help is appreciated.

### Code Generation

To maintain consistency with the original library, this project uses pre-compiled metadata. If you need to regenerate the metadata, for instance, after updating the `PhoneNumberMetadata.xml` file, you can use the provided tools.

The `tools` directory contains a rewritten Rust-based code generator for the C++ pre-compiled metadata.

To run the code generation process, execute the following script:

```sh
./tools/scripts/generate_metadata.sh
```

This script will:
1.  Build the Java-based tool that converts the XML metadata to a Rust-compatible format.
2.  Run the generator for the main metadata and the test metadata.
3.  Place the generated `.rs` files into the `src/generated/metadata` directory.

You can skip the Java build step by passing the `--skip-install` flag, which is useful if no changes were made to the generator itself.

```sh
./tools/scripts/generate_metadata.sh --skip-install
```

## License

This project is licensed under the Apache License, Version 2.0. Please see the `LICENSE` file for details.
