
# Rlibphonenumber

[![Crates.io](https://img.shields.io/crates/v/rlibphonenumber.svg)](https://crates.io/crates/rlibphonenumber)
[![Docs.rs](https://docs.rs/rlibphonenumber/badge.svg)](https://docs.rs/rlibphonenumber)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A zero-allocation, high-performance Rust port of Google's `libphonenumber` library for parsing, formatting, and validating international phone numbers. 

**Used metadata version: v9.0.25**  
**Version:** `1.0.1`  
**Base libphonenumber:** `9.0.8`  
**Min supported Rust version:** `1.88.0`

## 🛡️ Correctness

Through over **11.2 million iterations** of malformed, randomized, and edge-case inputs, this library has proven **zero mismatches** in parsing, validation rules (`is_valid`, `is_possible`), and formatting outputs (E.164, National, International, RFC3966) compared to the upstream C++ implementation. It provides exact drop-in behavior with Rust's memory safety and high execution speed.

## Performance

Performance is measured using `criterion`. We compare `rlibphonenumber` with the popular `rust-phonenumber` (the `phonenumber` crate) and `phonelib` crates.

All benchmarks measure the average time required to process a **single phone number**.

### Initialization
`rlibphonenumber` requires initializing `PhoneNumberUtil`, which loads the necessary metadata. This is typically done once at application startup:
* **`PhoneNumberUtil::new()`**: ~6.41 ms

### Parsing
Time required to parse a string representation into a phone number object:

| Library | `parse()` | Notes |
|---|---|---|
| **`rlibphonenumber`** | **~568 ns** | **Fastest & most reliable** |
| `rust-phonenumber` | ~832 ns | Fails on certain valid numbers.* |
| `phonelib` | *Failed* | Fails on certain valid numbers. |

*\* During testing, we found that `rust-phonenumber` (`rlp`) returns an error on valid phone numbers, such as the Brazilian number `"+55 11 98765-4321"`.*

### Formatting
Time required to format a parsed phone number object into various standards:

| Format | `rlibphonenumber` | `rust-phonenumber` | `phonelib` |
|---|---|---|---|
| **E164** | **~33.7 ns** 🚀 | ~721 ns | ~710 ns |
| **International** | **~423 ns** | ~1.01 µs | ~772 ns |
| **National** | **~562 ns** | ~1.38 µs | ~752 ns |
| **RFC3966** | **~587 ns** | ~1.18 µs | ~906 ns |

### Under the Hood: How is it so fast?
* **Zero-Allocation Formatting:** Intermediate heap allocations are eliminated. By utilizing `Cow<str>`, stack-allocated buffers (via a custom zero-padding `itoa` implementation), and a specialized Builder pattern, formatting numbers rarely touches the system allocator.
* **Build-Time Anchored Regexes (`RegexTriplets`):** Instead of allocating strings at runtime to wrap patterns in `^(?:...)$`, a custom Java build script pre-compiles and wraps metadata directly into the Protobuf output. At runtime, Rust uses `[..]` string slicing (zero-cost) to extract exact bounds, bypassing the regex engine's O(N) linear search and forcing `O(1)` fast-fail anchor matching.
* **Fast Hashing:** Replaces the default `SipHash` with `FxHash` (`rustc_hash`) for ultra-low-latency metadata lookups by region code and integer keys.
* **Lazy Initialization:** Regular expressions are compiled lazily and cached on-demand directly inside metadata wrappers using `std::sync::OnceLock`, removing the locking overhead of a centralized regex cache.

## Installation

Add `rlibphonenumber` to your `Cargo.toml`:

```toml
[dependencies]
rlibphonenumber = "1.0.2"
```

### Enabling Serde

To enable `Serialize` and `Deserialize` support for `PhoneNumber`:

```toml
[dependencies]
rlibphonenumber = { version = "1.0.0", features = ["serde"] }
```

## Getting Started

The library exposes a global static `PHONE_NUMBER_UTIL`, but for most common operations, you can use methods directly on the `PhoneNumber` struct.

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

## Differential Fuzzing

We invite anyone to verify our correctness parity. The repository includes a Dockerized environment that links Google's C++ `libphonenumber` side-by-side with our Rust implementation via `cxx`.

To run the differential fuzzer locally:

1. Clone the repository and open the provided DevContainer/Docker environment.
2. Run the `full-cycle` fuzz target to check fully random user inputs and ensure no panics occur:
   ```sh
   cargo +nightly fuzz run full-cycle
   ```
3. Run the `diff-test` target to compare outputs with the original library (requires the C++ library version to match the metadata version used):
   ```sh
   cargo +nightly fuzz run diff-test
   ```

If the fuzzer ever finds a single input where the Rust output deviates from the C++ output, it will immediately crash and save the artifact.

## Manual Instantiation & Feature Flags

By default, this crate enables the `global_static` feature, which initializes a thread-safe, lazy-loaded static instance `PHONE_NUMBER_UTIL`. This allows you to use convenience methods directly on `PhoneNumber`.

If you need granular control over memory usage, wish to avoid global state, or are working in a strict environment, you can disable this feature.

```toml
[dependencies]
rlibphonenumber = { version = "1.0.0", default-features = false }
```

When `global_static` is disabled, helper methods on `PhoneNumber` (like `.format_as()`, `.is_valid()`) **will not be available**. You must instantiate the utility manually.

**⚠️ Performance Note:** `PhoneNumberUtil::new()` compiles regexes upon initialization. This is an expensive operation. Create it once and reuse it (e.g., wrap it in an `Arc` or pass it by reference).

```rust
use rlibphonenumber::{PhoneNumberUtil, PhoneNumber};

fn main() {
    // 1. Initialize the utility once
    let phone_util = PhoneNumberUtil::new();

    let number_str = "+15550109988";

    // 2. Parse using the instance
    if let Ok(number) = phone_util.parse(number_str, None) {
        // 3. Use the instance for validation
        let is_valid = phone_util.is_valid_number(&number).unwrap_or(false);
        println!("Valid: {}", is_valid);
    }
}
```
