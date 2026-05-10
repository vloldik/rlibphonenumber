# Rlibphonenumber cli (rpn)

A command-line interface for [rlibphonenumber](https://github.com/vloldik/rlibphonenumber) — a Rust port of Google's [libphonenumber](https://github.com/google/libphonenumber) Java library.

Parse, validate, find, and mask phone numbers from files, URLs, SSH paths, and Git repositories.

---

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/vloldik/rlibphonenumber/main/workspaces/rlibphonenumber/crates/rlibphonenumber_cli/install.sh | bash
```

You can use `cargo install`

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
./target/release/rpn --help
```

---

## Commands

### `metadata` — Build and validate phone number metadata

Shared flags apply to both `build` and `validate` subcommands:

| Flag | Short | Description |
|---|---|---|
| `--input` | `-i` | Input source (XML or binary `.bin`) |
| `--filter` | `-f` | CEL expression to filter metadata fields (XML only) |
| `--alternate-formats` | | Relax common metadata rules for alternate formats |

#### `metadata build <output_dir> <basename>`

Compile an XML metadata file into a binary `.bin` artifact, and optionally emit a Rust module.

```bash
# Basic build
rpn metadata -i PhoneNumberMetadata.xml build ./out core

# Skip validation, emit a Rust module with a custom constant name
rpn metadata -i PhoneNumberMetadata.xml build ./out core \
  --const-name PHONE_METADATA \
  --skip-validate \
  --generate-mod
```

| Flag | Short | Default | Description |
|---|---|---|---|
| `--const-name` | | `METADATA` | Name of the exported Rust `static` constant |
| `--skip-validate` | | false | Skip post-build metadata validation |
| `--generate-mod` / `-m` | | false | Also emit a `.rs` file with an `include_bytes!` constant |

Output files written to `<output_dir>`:
- `<basename>.bin` — encoded protobuf metadata
- `<basename>.rs` — Rust module (if `--generate-mod` is set)

#### `metadata validate`

Validate an existing XML or binary metadata file without producing any output files.

```bash
rpn metadata -i PhoneNumberMetadata.xml validate
rpn metadata -i core.bin validate
rpn metadata -i PhoneNumberMetadata.xml --alternate-formats validate
```

---

### `number` — Parse, find, and mask phone numbers

Shared flags apply to all `number` subcommands:

| Flag | Short | Description |
|---|---|---|
| `--region` | `-r` | Default region code (e.g. `US`, `DE`, `RU`) |
| `--metadata` | `-m` | Path to a custom `.bin` metadata file |

#### `number parse <number>`

Parse a single phone number and display its properties.

```bash
rpn number parse "+49 30 12345678"
rpn number -r DE parse "030 12345678" --output wide
rpn number parse "+12025551234" --output json --format international
```

| Flag | Short | Default | Description |
|---|---|---|---|
| `--output` | `-o` | `plaintext` | Output mode: `plaintext`, `json`, `wide` |
| `--format` | `-f` | `e164` | Number format: `e164`, `international`, `national`, `rfc3966` |

#### `number find <input>`

Stream text from a source and extract all phone numbers found in it.

```bash
rpn number find ./contacts.txt
rpn number find https://example.com/data.txt --output json
rpn number -r US find ./emails.txt --leniency possible --max-tries 1000
```

| Flag | Short | Default | Description |
|---|---|---|---|
| `--output` | `-o` | `plaintext` | Output mode: `plaintext`, `json`, `wide` |
| `--format` | `-f` | `e164` | Number format: `e164`, `international`, `national`, `rfc3966` |
| `--leniency` | `-l` | `valid` | Parser leniency: `possible`, `valid`, `strict_grouping`, `exact_grouping` |
| `--max-tries` | `-t` | unlimited | Maximum number of matches to attempt |
| `--window-size` | `-w` | `65536` | Sliding window size in bytes |
| `--overlap` | `-v` | `1024` | Window overlap size in bytes |

#### `number mask <type> <input>`

Stream text from a source, replacing every detected phone number with a masked version. Four masking strategies are available:

```bash
# Replace all numbers with a fixed string
rpn number mask constant ./log.txt "[REDACTED]"

# Replace with a reversible semantic token (e.g. PHONE_XX_XXXXXXXX#<hash>)
rpn number mask token ./log.txt
rpn number mask token ./log.txt --without-hash   # no HMAC suffix

# Replace with an HMAC-SHA256 hash
rpn number mask hash ./log.txt
rpn number mask hash ./log.txt --prefix "ph:"

# Format then partially mask digits
rpn number mask mask ./log.txt
rpn number mask mask ./log.txt --format international --mask-char '#' --min-masked 6
```

Shared `mask` flags:

| Flag | Short | Default | Description |
|---|---|---|---|
| `--leniency` | `-l` | `valid` | Parser leniency |
| `--max-tries` | `-t` | unlimited | Maximum matches to attempt |
| `--window-size` | `-w` | `65536` | Sliding window size in bytes |
| `--overlap` | `-v` | `1024` | Window overlap size in bytes |

`mask mask` specific flags:

| Flag | Short | Default | Description |
|---|---|---|---|
| `--format` | `-f` | e164 | Reformat before masking: `e164`, `international`, etc. |
| `--mask-char` | `-m` | `*` | Character used to replace masked digits |
| `--min-masked` | `-c` | `4` | Minimum number of digits to mask |
| `--max-unmasked` | `-u` | `4` | Maximum digits left visible at the end |

> **`hash` and `token` masking use HMAC-SHA256.**
> Set the `PHONE_HMAC_KEY` environment variable to ensure deterministic, reproducible output across runs.
> If unset, a random key is generated per session and a warning is printed to stderr.

---

## Input Sources

All commands that accept an input path support the following source formats:

| Format | Example |
|---|---|
| Local file | `path/to/file.xml` |
| HTTP/HTTPS URL | `https://example.com/data.bin` |
| SSH path | `user@host:/path/to/file` |
| Git repository | `git://github.com/user/repo.git?file=data.xml&branch=main` |

---

## Metadata Filtering (CEL)

The `--filter` flag on the `metadata` command accepts a [CEL](https://github.com/google/cel-spec) expression to selectively drop metadata fields during a build or validation pass.

The following variables are available in the expression:

| Variable | Type | Description |
|---|---|---|
| `region` | `string` | Region identifier (e.g. `"US"`) |
| `country_code` | `int` | Numeric country code |
| `is_main_country` | `bool` | Whether this is the main country for the dialing code |
| `parent` | `string \| null` | Parent descriptor name (e.g. `"mobile"`) |
| `field` | `string \| null` | Field name being evaluated (e.g. `"example_number"`) |

The expression must return a `bool`. Return `true` to **keep** a field, `false` to **drop** it.

```bash
# Drop example numbers from all descriptors
rpn metadata -i PhoneNumberMetadata.xml \
  --filter 'field != "example_number"' \
  build ./out lite

# Keep only US and CA regions
rpn metadata -i PhoneNumberMetadata.xml \
  --filter 'region == "US" || region == "CA"' \
  build ./out north_america
```

---

## License

Same as the parent `rlibphonenumber` crate.