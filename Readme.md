# Rlibphonenumber

[![Crates.io](https://img.shields.io/crates/v/rlibphonenumber.svg)](https://crates.io/crates/rlibphonenumber)
[![Docs.rs](https://docs.rs/phonenumber/badge.svg)](https://docs.rs/rlibphonenumber)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A Rust port of Google's comprehensive library for parsing, formatting, and validating international phone numbers.

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

This significant performance advantage is achieved through a focus on minimizing allocations, a more direct implementation path, and the use of modern tooling for metadata generation.

## Current Status

The project is currently in its initial phase of development. The core functionalities are being ported module by module to ensure quality and consistency.

### Implemented:
*   **PhoneNumberUtil:** The main utility for all phone number operations, such as parsing, formatting, and validation (Passes original tests).

### Future Plans:
The roadmap includes porting the following key components:

*   **AsYouTypeFormatter:** To format phone numbers as they are being typed.
*   **PhoneNumberOfflineGeocoder:** To provide geographical information for a phone number.
*   **PhoneNumberToCarrierMapper:** To identify the carrier associated with a phone number.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rlibphonenumber = "0.1.0" # Replace with the actual version
```

## Getting Started

Here is a basic example of how to parse and format a phone number:

```rust
use rlibphonenumber::{PhoneNumberFormat, PHONE_NUMBER_UTIL};

fn main() {
    let number_to_parse = "+14155552671";
    let default_region = "US";

    match PHONE_NUMBER_UTIL.parse(number_to_parse, default_region) {
        Ok(number) => {
            println!("Parsed number: {:?}", number);

            let formatted_number = PHONE_NUMBER_UTIL.format(&number, PhoneNumberFormat::International).unwrap();
            println!("International format: {}", formatted_number);

            let is_valid = PHONE_NUMBER_UTIL.is_valid_number(&number).unwrap();
            println!("Is the number valid? {}", is_valid);
        }
        Err(e) => {
            println!("Error parsing number: {:?}", e);
        }
    }
}
```

## For Contributors

Contributions are **highly** welcome! Whether you are fixing a bug, improving documentation, or helping to port a new module, your help is appreciated.

### Code Generation

To maintain consistency with the original library, this project uses pre-compiled metadata. If you need to regenerate the metadata, for instance, after updating the `PhoneNumberMetadata.xml` file, you can use the provided tools.

The `tools` directory contains a rewritten Rust-based code generator for the C++ pre-compiled metadata.

To run the code generation process, simply execute the following script:

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
