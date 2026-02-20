use std::borrow::Cow;
use std::mem::MaybeUninit;
use std::{ptr, slice, str};

const DEC_DIGITS_LUT: [u8; 200] = *b"\
      0001020304050607080910111213141516171819\
      2021222324252627282930313233343536373839\
      4041424344454647484950515253545556575859\
      6061626364656667686970717273747576777879\
      8081828384858687888990919293949596979899";

pub struct LeadingZeroBuffer {
    bytes: [MaybeUninit<u8>; 64],
}

impl Default for LeadingZeroBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LeadingZeroBuffer {
    pub fn new() -> Self {
        Self {
            bytes: [MaybeUninit::uninit(); 64],
        }
    }

    /// Formats a `u64` integer into a string with a specified number of leading zeros.
    ///
    /// This method uses a backward-filling algorithm with a digit look-up table
    /// for maximum performance.
    ///
    /// # Arguments
    ///
    /// * `n` - The 64-bit unsigned integer to format.
    /// * `leading_zero_count` - The number of '0' characters to prepend to the number.
    ///
    /// # Returns
    ///
    /// Returns a [`Cow<'_, str>`]:
    /// - [`Cow::Borrowed`]: If the formatted string and leading zeros fit within the
    ///   internal 64-byte buffer.
    /// - [`Cow::Owned`]: If the `leading_zero_count` is large enough that the
    ///   total length exceeds the available buffer space.
    ///
    /// # Safety
    ///
    /// This function uses `unsafe` blocks to:
    /// - Perform direct pointer arithmetic and memory copies for speed.
    /// - Convert a byte slice to a `&str` without UTF-8 validation (guaranteed by
    ///   the decimal conversion logic).
    /// - Access uninitialized memory in the internal buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use zeroes_itoa::LeadingZeroBuffer;
    /// let mut buffer = LeadingZeroBuffer::new();
    ///
    /// // Borrowed from stack buffer
    /// let s = buffer.format(42, 3);
    /// assert_eq!(s, "00042");
    ///
    /// // Owned String (if padding is very large)
    /// let s = buffer.format(7, 100);
    /// assert_eq!(s.len(), 101);
    /// ```
    pub fn format(&mut self, mut n: u64, leading_zero_count: usize) -> Cow<'_, str> {
        let mut curr = self.bytes.len();
        let buf_ptr = self.bytes.as_mut_ptr() as *mut u8;
        let lut_ptr = DEC_DIGITS_LUT.as_ptr();

        while n >= 10000 {
            let rem = n % 10000;
            n /= 10000;

            let d1 = ((rem / 100) << 1) as usize;
            let d2 = ((rem % 100) << 1) as usize;
            curr -= 4;
            unsafe {
                ptr::copy_nonoverlapping(lut_ptr.add(d1), buf_ptr.add(curr), 2);
                ptr::copy_nonoverlapping(lut_ptr.add(d2), buf_ptr.add(curr + 2), 2);
            }
        }

        if n >= 100 {
            let d1 = ((n % 100) << 1) as usize;
            n /= 100;
            curr -= 2;
            unsafe {
                ptr::copy_nonoverlapping(lut_ptr.add(d1), buf_ptr.add(curr), 2);
            }
        }
        if n < 10 {
            curr -= 1;
            unsafe {
                *buf_ptr.add(curr) = (n as u8) + b'0';
            }
        } else {
            let d1 = (n << 1) as usize;
            curr -= 2;
            unsafe {
                ptr::copy_nonoverlapping(lut_ptr.add(d1), buf_ptr.add(curr), 2);
            }
        }

        if leading_zero_count > curr {
            let final_len = self.bytes.len() - curr;
            let bytes = unsafe { slice::from_raw_parts(buf_ptr.add(curr), final_len) };
            let string: &str = unsafe { str::from_utf8_unchecked(bytes) };

            let mut prefixed_string = String::with_capacity(string.len() + leading_zero_count);
            (0..leading_zero_count).for_each(|_| prefixed_string.push('0'));
            prefixed_string.push_str(string);
            prefixed_string.into()
        } else {
            curr -= leading_zero_count;

            unsafe {
                ptr::write_bytes(buf_ptr.add(curr), b'0', leading_zero_count);
            }
            let final_len = self.bytes.len() - curr;
            let bytes = unsafe { slice::from_raw_parts(buf_ptr.add(curr), final_len) };
            unsafe { str::from_utf8_unchecked(bytes).into() }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::LeadingZeroBuffer;

    #[test]
    fn test_mod() {
        const STEP: u64 = u64::MAX / 1000;
        for zero_count in 0..1000 {
            for number in 0..=(u64::MAX / STEP) {
                let number = number * STEP;
                let mut buf = LeadingZeroBuffer::new();
                assert_eq!(
                    buf.format(number, zero_count),
                    format!("{}{}", "0".repeat(zero_count), number)
                )
            }
        }
    }
}
