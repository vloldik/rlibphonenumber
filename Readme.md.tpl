# Rlibphonenumber v2

[![Crates.io](https://img.shields.io/crates/v/rlibphonenumber.svg)](https://crates.io/crates/rlibphonenumber)
[![Docs.rs](https://docs.rs/rlibphonenumber/badge.svg)](https://docs.rs/rlibphonenumber)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![WASM Preview](https://img.shields.io/badge/Live-WASM_Preview-success.svg)](https://vloldik.github.io/rlibphonenumber-wasm/)

A zero-allocation, high-performance Rust port of Google's `libphonenumber` library for parsing, formatting, extracting, and validating international phone numbers. 

**Used metadata version:** `{{metadata_version}}`  
**Package version**: `{{package_version}}`
**Base libphonenumber:** `9.0.8`  
**Min supported Rust version:** `1.88.0`

---

## 🚀 What's New in v2 (Migration Guide & Breaking Changes)

Version 2 brings a completely redesigned core, shedding legacy implementations in favor of idiomatic, zero-cost Rust abstractions.

*   **Migrated from `rust-protobuf` to `prost`**: The internal representation now uses `prost`, resulting in a smaller footprint, faster decoding, and more idiomatic Rust types.
*   **Unified `parse` API with `Region` Enum**: `parse` and `parse_with_region` have been merged. The API **no longer accepts string slices** for regions. You must now pass a strictly typed `Region` enum (e.g., `Region::US`).
*   **O(1) Branchless Region Parsing**: The `Region` enum is generated at compile-time using bitwise shifts (mapping 2-letter ASCII codes to 16-bit discriminants). Parsing `"US"` into `Region::US` now takes exactly 1 CPU cycle without a single `match` branch or `if/else`. Generating a string back is done via a zero-allocation, 4-byte stack structure (`RegionStr`).
*   **Redesigned Public API Wrapper**: We implemented a custom procedural macro that generates a clean, infallible public API while keeping the complex generic and lifetime-heavy implementations completely internal.
*   **AOT Metadata Validation**: Custom metadata is now strictly validated at compile time (checking lengths < 64, compiling all regexes to prevent runtime panics).
*   **Initialization Speedup**: Bootstrapping `PhoneNumberUtil::new()` is now **~10% faster**, taking only **~4.97 ms**.

## ✨ Enterprise Features

### 🔍 Streaming Matcher (Number Extraction)
*   **Exact Grouping Leniency:** Validates not just the digits, but whether the user formatted the number exactly according to the country's telecom rules (e.g., rejecting `12-34-567-890` while accepting `(123) 456-7890`).
*   **Extension Traits:** Simply call `"Call +1 555-0199".find_phone_numbers()` to start extracting.
*   *Correctness:* The matcher has passed **500,000 iterations of Differential Fuzzing** directly against Google's C++ ICU implementation with zero mismatches.

### 🛡️ Data Loss Prevention (Masking & Hashing)
The new `PhoneMaskUtil` is designed for GDPR/PII compliance in high-throughput environments:
*   **Zero-Allocation Pipeline:** Uses a custom `LenWrite` trait to predict output lengths and write masked numbers or XML tokens directly into `stdout` or file buffers without heap allocations.
*   **Cryptographic Hashing:** Supports `HMAC` and `SHA256` hashing directly into stack-allocated 64-byte arrays.
*   **Smart Obfuscation:** Automatically detects and fully masks RFC3966 URIs and phone extensions, leaving only the requested digits visible (e.g., `***-***-1234`).

## ⚙️ CI/CD & Dagger Pipelines

The repository is fully automated using **Dagger** (Infrastructure as Code). Our pipelines automatically:
1. Fetch the latest `v9.0.x` XML metadata from Google.
2. Compile and validate the regexes.
3. Perform Differential Fuzzing against a compiled C++ container.
4. Auto-bump crate versions.

---

## 📦 Installation & Feature Flags

Add `rlibphonenumber` to your `Cargo.toml`:

```toml
[dependencies]
rlibphonenumber = "{{package_version}}"
```

### Available Features

| Feature | Description | Default |
|---|---|---|
| `builtin_metadata` | Embeds the compiled `.bin` metadata into the binary. **Required for `global_static`.** | ✅ |
| `global_static` | Enables the lazy-loaded global `PHONE_NUMBER_UTIL` and `FindNumberExt` string traits. | ✅ |
| `regex` | Uses the standard `regex` crate for maximum speed. | ✅ |
| `lite` | Uses `regex-lite`. Optimizes for binary size (ideal for WASM/Embedded). | ❌ |
| `digest` | Enables cryptographic hashing of phone numbers (e.g., SHA256) into stack buffers. | ❌ |
| `digest_mac` | Enables keyed hashing (HMAC) for phone numbers. Depends on `digest`. | ❌ |
| `serde` | Enables `Serialize`/`Deserialize` for `PhoneNumber`. | ❌ |

---

## 🛠️ CLI & Custom Metadata Management

`rlibphonenumber` includes a powerful CLI for masking files on the fly and compiling custom metadata (e.g., filtering out pager rules via CEL expressions to shrink binary size).

📖 **[Read the dedicated CLI Documentation here.](./crates/rlibphonenumber_cli/Readme.md)**

---

## 🚀 Getting Started

### Parsing & Formatting
```rust
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumber, PhoneNumberFormat, enums::Region};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse the number (v2 requires the Region enum)
    let number = PHONE_NUMBER_UTIL.parse("555-0199", Some(Region::US))?;

    // 2. Validate
    if number.is_valid() {
        // 3. Format
        println!("E.164: {}", number.format_as(PhoneNumberFormat::E164)); // +15550199
    }

    Ok(())
}
```

### Finding Numbers in Text (Matcher)
```rust
use rlibphonenumber::phonenumber_matcher::FindNumberExt;

fn main() {
    let text = "Contact us at +1 (202) 555-0173 or drop a fax at 020 7183 8750.";
    
    // Extension trait directly on &str
    for match_result in text.find_phone_numbers() {
        println!("Found: {} at index {}", match_result.number, match_result.start);
    }
}
```

### High-Performance Masking & Hashing
*(Requires `digest_mac` feature)*
```rust
use rlibphonenumber::{PHONE_NUMBER_UTIL, phonenumber_mask::{PhoneMaskUtil, MaskDigitsConfig, PhoneMacHasher}};
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn main() {
    let mask_util = PhoneMaskUtil::new();
    let number = PHONE_NUMBER_UTIL.parse("+12025550173", None).unwrap();

    // 1. Partial Masking (***-***-0173)
    let config = MaskDigitsConfig::new('*', 4, 4); // mask at least 4, leave last 4
    let masked = mask_util.mask_digits_to_string("+1 202-555-0173 ext. 89", config);
    println!("Masked: {}", masked);

    // 2. Semantic Tokenization with HMAC
    let mut mac = Hmac::<Sha256>::new_from_slice(b"my_secret_salt").unwrap();
    let token = mask_util.tokenize_to_string(&number, PhoneMacHasher(mac)).unwrap();
    
    // <Phone country="US" hash="a1b2c3d4...">
    println!("Token: {}", token); 
}
```

---

## ⚡ Performance

Benchmarks use `criterion` measuring the average time to process a **single phone number** using native toolchains (C++ `google/benchmark` with RE2 vs Rust `rlibphonenumber`). 

Both benchmarks bypass CPU branch-predictor memorization.

| Operation | C++ (`libphonenumber` + RE2) | Rust (`rlibphonenumber`) | Speedup |
| :--- | :--- | :--- | :--- |
| **Parsing** | ~2.28 µs *(2279 ns)* | **~0.50 µs *(500 ns)*** | **~4.5x** |
| **Format (E.164)** | ~63 ns | **~33 ns** | **~1.9x** |
| **Format (International)** | ~2.03 µs *(2028 ns)* | **~0.43 µs *(432 ns)*** | **~4.7x** |
| **Format (National)** | ~2.48 µs *(2484 ns)* | **~0.56 µs *(558 ns)*** | **~4.4x** |
| **Format (RFC3966)** | ~2.42 µs *(2417 ns)* | **~0.61 µs *(606 ns)*** | **~4.0x** |

### Under the Hood: Why is it so fast?
* **Zero-Allocation Formatter:** Intermediate heap allocations are eliminated using `Cow<str>` and stack-allocated zero-padding buffers.
* **O(1) Pre-Anchored Regexes:** Instead of runtime string concatenation (`"^(?:" + pattern + ")$"`), validation metadata is compiled AOT (Ahead-of-Time). Rust uses `[..]` string slicing to fast-fail boundary checks, bypassing O(N) regex engine sweeps.
* **`FxHash` Maps:** We replaced standard `SipHash` with `rustc_hash` for ultra-low latency metadata lookups.
* **Lazy Compilation:** Regexes are compiled lazily inside the metadata wrappers via `OnceLock`, removing centralized cache contention.