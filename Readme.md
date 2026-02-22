
# Rlibphonenumber

[![Crates.io](https://img.shields.io/crates/v/rlibphonenumber.svg)](https://crates.io/crates/rlibphonenumber)
[![Docs.rs](https://docs.rs/rlibphonenumber/badge.svg)](https://docs.rs/rlibphonenumber)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Update Metadata & Push](https://github.com/vloldik/rlibphonenumber/actions/workflows/update-metadata.yaml/badge.svg)](https://github.com/vloldik/rlibphonenumber/actions/workflows/update-metadata.yaml)

A high-performance Rust port of Google's `libphonenumber` library for parsing, formatting, and validating international phone numbers.

## Min supported Rust version is 1.88.0

**Built on base libphonenumber 9.0.8**
**Used metadata version: v9.0.24**

## Overview

This library is a fresh adaptation of Google's `libphonenumber` for Rust. Its primary goal is to provide a powerful and efficient tool for handling phone numbers, with a structure that is intuitively close to the original C++ version, but adapted for Rust ergonomics.

Key capabilities include:
*   Parsing and formatting phone numbers.
*   Validating phone numbers for all regions of the world.
*   Determining the number type (e.g., Mobile, Fixed-line, Toll-free).
*   Serde support (optional).
*   Thread-safe and high performance.

## Performance & Architecture

`rlibphonenumber` is strictly designed for low latency and minimal memory overhead. The latest benchmarks show a significant performance advantage over existing alternatives.

| Operation | rlibphonenumber (this crate) | rust-phonenumber | Difference |
|:---|:---:|:---:|:---:|
| **Format (E164)** | **~262 ns** | ~9.20 µs | **~35x faster** |
| **Format (International)** | **~4.57 µs** | ~11.89 µs | **~2.6x faster** |
| **Format (National)** | **~6.03 µs** | ~15.30 µs | **~2.5x faster** |
| **Format (RFC3966)** | **~6.04 µs** | ~12.91 µs | **~2.1x faster** |
| **Parse** | **~6.12 µs** | ~8.55 µs | **~1.4x faster** |

### Under the Hood
The high performance is achieved through several architectural optimizations:
* **Zero-allocation formatting:** Extensive use of stack-allocated buffers (e.g., custom `itoa` with zero-padding), `Cow<str>`, and a specialized Builder pattern. This prevents intermediate heap allocations and unnecessary string concatenations during number formatting.
* **Fast Hashing:** Replaced default `SipHash` with `FxHash` (`rustc_hash`) for low-latency metadata lookups by region code and integer keys.
* **Lazy Regex Initialization:** Regular expressions are compiled and cached on-demand directly inside metadata wrappers using `std::sync::OnceLock`, eliminating the synchronization overhead of a centralized regex cache.
* **Static Dispatch:** Core matching and validation logic relies on monomorphization rather than dynamic dispatch (`dyn Trait`), allowing the compiler to aggressively inline execution paths.

## Installation

Add `rlibphonenumber` to your `Cargo.toml`:

```toml
[dependencies]
rlibphonenumber = "0.3.3"
```

### Enabling Serde

To enable `Serialize` and `Deserialize` support for `PhoneNumber`:

```toml
[dependencies]
rlibphonenumber = { version = "0.3.3", features = ["serde"] }
```

## Getting Started

The library exposes a global static `PHONE_NUMBER_UTIL`, but for most common operations, you can now use methods directly on the `PhoneNumber` struct.

### Complete Example

```rust
use rlibphonenumber::{
    PHONE_NUMBER_UTIL,
    PhoneNumber,
    PhoneNumberFormat,
    ParseError,
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
```

### Serde Integration

When the `serde` feature is enabled, `PhoneNumber` serializes to a string (E.164 format) and can be deserialized from a string.

```rust
use rlibphonenumber::PhoneNumber;
use serde_json::json;

fn main() {
    let raw = "+15875302271";
    let number: PhoneNumber = raw.parse().unwrap();

    // Serializes to "+15875302271"
    let json_output = json!({ "phone": number });
    println!("{}", json_output); 
}
```

## Project Status

The library is under active development. The core `PhoneNumberUtil` is fully implemented and passes the original library's test suite.

Roadmap:
*   **`AsYouTypeFormatter`**: For formatting phone numbers as a user types.
*   **`PhoneNumberOfflineGeocoder`**: To provide geographical location information.
*   **`PhoneNumberToCarrierMapper`**: To identify the carrier associated with a number.

## Contributing

Contributions are highly welcome! Whether you are fixing a bug, improving documentation, or helping to port a new module, your help is appreciated.

### Code Generation

To maintain consistency with the original library, this project uses pre-compiled metadata. If you need to regenerate the metadata (e.g., after updating `PhoneNumberMetadata.xml`), use the provided script:

```sh
./tools/scripts/generate_metadata.sh --tag v9.0.23
```

## Fuzz Testing

Beyond standard unit tests that cover expected behavior, `rlibphonenumber` undergoes rigorous fuzz testing to ensure its resilience against unexpected, malformed, and potentially malicious input. Given that phone number parsing often deals with untrusted data from users, stability is a primary design goal.

### Running the Fuzzer

1.  **Install prerequisites:**
    ```sh
    rustup default nightly
    cargo install cargo-fuzz
    ```

2.  **Run a fuzz target:**
    *   `full-cycle`: A comprehensive test of the parse -> validate -> format workflow.

    To start a fuzzing session, run:
    ```sh
    # Example for the main target
    cargo fuzz run full-cycle
    ```

### Manual Instantiation & Feature Flags

By default, this crate enables the `global_static` feature, which initializes a thread-safe, lazy-loaded static instance `PHONE_NUMBER_UTIL`. This allows you to use convenience methods directly on `PhoneNumber` (e.g., `number.is_valid()`).

However, if you need granular control over memory usage, wish to avoid global state, or are working in an environment where lazy statics are undesirable, you can disable this feature.

#### Disabling the Global Instance

In your `Cargo.toml`, disable the default features:

```toml
[dependencies]
rlibphonenumber = { version = "0.3.1", default-features = false }
```

#### Using `PhoneNumberUtil::new()`

When `global_static` is disabled, the `PHONE_NUMBER_UTIL` constant and the helper methods on `PhoneNumber` (like `.format_as()`, `.is_valid()`) **will not be available**. You must instantiate the utility manually and pass it around.

**⚠️ Performance Note:** `PhoneNumberUtil::new()` compiles regexes upon initialization. This is an expensive operation. Create it once and reuse it (e.g., wrap it in an `Arc` or pass it by reference).

```rust
use rlibphonenumber::{PhoneNumberUtil, PhoneNumber};

fn main() {
    // 1. Initialize the utility once
    let phone_util = PhoneNumberUtil::new();

    let number_str = "+15550109988";

    // 2. Parse using the instance
    // Note: 'parse' is a method on phone_util, not a trait on str here
    match phone_util.parse(number_str, None) {
        Ok(number) => {
            // 3. Use the instance for validation and formatting
            // number.is_valid() is NOT available without 'global_static'
            let is_valid = phone_util.is_valid_number(&number).unwrap_or(false);
            
            println!("Valid: {}", is_valid);
        }
        Err(e) => eprintln!("Parse error: {:?}", e),
    }
}
```

## License

This project is licensed under the Apache License, Version 2.0. Please see the `LICENSE` file for details.