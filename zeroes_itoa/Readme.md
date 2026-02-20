# LeadingZeroBuffer

A high-performance integer-to-string formatter for Rust, specifically designed to handle **leading zero padding** with minimal allocations.

## 💡 About this Fork

This implementation is a specialized fork of the logic found in the popular [`itoa`](https://github.com/dtolnay/itoa) crate by David Tolnay. 

While the original `itoa` is optimized for writing integers directly to a writer or a buffer as quickly as possible, **LeadingZeroBuffer** is adapted for cases where you need:
1.  **Variable leading zeros**: Prepending a dynamic number of `'0'` characters.
2.  **Smart Allocation**: It uses a `Cow<'_, str>` strategy. If the number plus the padding fits within the internal 64-byte stack buffer, it returns a borrowed `&str`. If the padding is exceptionally large, it transparently falls back to an owned `String`.

## 🛠 Usage

```rust
let mut buffer = LeadingZeroBuffer::new();

// Example 1: Fits in the buffer (Returns Cow::Borrowed)
// "42" with 3 leading zeros -> "00042"
let borrowed = buffer.format(42, 3);
assert_eq!(borrowed, "00042");

// Example 2: Massive padding (Returns Cow::Owned)
let owned = buffer.format(7, 100);
assert_eq!(owned.len(), 101);
assert!(owned.starts_with("000000"));
```

## ❓ Why not just use `format!("{:0>width$}", n)`?

The standard library `format!` macro:
1.  Always allocates a new `String` on the heap.
2.  Involves overhead from parsing the format string at runtime.
3.  Is significantly slower in tight loops (e.g., generating timestamps, sequence numbers, or log entries).

`LeadingZeroBuffer` provides a "zero-allocation" path for the vast majority of use cases while maintaining the speed of the `itoa` algorithm.

## ⚖️ License

This project incorporates logic from `itoa`, which is dual-licensed under the MIT and Apache 2.0 licenses. This fork maintains the same licensing terms.