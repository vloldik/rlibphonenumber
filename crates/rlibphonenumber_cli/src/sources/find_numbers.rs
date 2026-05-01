use std::collections::HashSet;
use std::io::Read;

use rlibphonenumber::PhoneNumber;
use rlibphonenumber::phonenumber_matcher::PhoneNumberMatch;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FoundNumber {
    pub number: PhoneNumber,
    pub start: usize,
    pub len: usize,
}

impl From<PhoneNumberMatch<'_>> for FoundNumber {
    fn from(value: PhoneNumberMatch<'_>) -> Self {
        Self {
            number: value.number,
            start: value.start,
            len: value.raw_string.len(),
        }
    }
}

impl super::Source {
    /// Searches for phone numbers in the source using a sliding window.
    ///
    /// This method is maximally efficient:
    /// - Uses a single, fixed-size heap allocation (`Vec<u8>`).
    /// - Deduplicates on the fly using zero-allocation checks.
    /// - Handles UTF-8 boundaries and partial characters safely.
    ///
    /// # Arguments
    /// * `window_size` - The size of the sliding window in bytes (e.g., 64 * 1024).
    /// * `overlap` - The number of bytes to overlap between chunks to prevent splitting numbers (e.g., 1024).
    /// * `extract_matches` - A closure called with each valid UTF-8 text chunk. It should invoke `yield_match` for every found number candidate.
    pub fn search_phone_numbers<F>(
        &self,
        window_size: usize,
        overlap: usize,
        mut extract_matches: F,
    ) -> Result<HashSet<FoundNumber>, super::SourceReadError>
    where
        // Callback takes the valid text chunk and a `yield_match` function to push found strings
        F: FnMut(&str, &mut dyn FnMut(PhoneNumberMatch<'_>)),
    {
        let mut reader = self.read()?;
        let capacity = window_size.max(overlap * 2);
        let mut buffer = vec![0u8; capacity];
        let mut tail_len = 0;
        let mut absolute_offset = 0usize;

        let mut seen_numbers = HashSet::<FoundNumber>::new();

        loop {
            let mut yield_match = |candidate: PhoneNumberMatch| {
                let mut found = FoundNumber::from(candidate);
                found.start += absolute_offset;

                seen_numbers.insert(found);
            };
            let read_target = &mut buffer[tail_len..capacity];
            let mut bytes_read = 0;

            let mut temp_target = read_target;
            while !temp_target.is_empty() {
                let n = reader.read(temp_target)?;
                if n == 0 {
                    break;
                }
                bytes_read += n;
                let temp = temp_target;
                temp_target = &mut temp[n..];
            }

            let current_chunk_len = tail_len + bytes_read;
            if current_chunk_len == 0 {
                break; // EOF
            }

            // 2. Ensure we don't slice in the middle of a UTF-8 character
            let (valid_text, valid_len) = match std::str::from_utf8(&buffer[..current_chunk_len]) {
                Ok(s) => (s, current_chunk_len),
                Err(e) => {
                    let len = e.valid_up_to();
                    // It is mathematically guaranteed to be valid UTF-8 up to `len`
                    (std::str::from_utf8(&buffer[..len]).unwrap(), len)
                }
            };

            extract_matches(valid_text, &mut yield_match);
            if bytes_read == 0 {
                // We've processed the final chunk and reached EOF
                break;
            }

            let overlap_start = valid_len.saturating_sub(overlap);
            let mut actual_start = overlap_start;
            while actual_start < valid_len && !valid_text.is_char_boundary(actual_start) {
                actual_start += 1;
            }

            let keep_len = valid_len - actual_start;
            buffer.copy_within(actual_start..valid_len, 0);

            let leftover_bytes = current_chunk_len - valid_len;
            if leftover_bytes > 0 {
                buffer.copy_within(valid_len..current_chunk_len, keep_len);
            }

            // Update tail_len for the next read cycle
            tail_len = keep_len + leftover_bytes;
            absolute_offset += actual_start;
        }

        Ok(seen_numbers)
    }
}
