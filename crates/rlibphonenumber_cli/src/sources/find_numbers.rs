use std::io::Read;

use rlibphonenumber::phonenumber_matcher::PhoneNumberMatch;

use crate::sources::{FoundNumber, FoundToken, SearchNumbers};

impl From<PhoneNumberMatch<'_>> for FoundNumber {
    fn from(value: PhoneNumberMatch<'_>) -> Self {
        Self {
            number: value.number,
            start: value.start,
            len: value.raw_string.len(),
        }
    }
}

impl<T: super::ReadSource> SearchNumbers for T {
    /// Searches for phone numbers and streams both the numbers and the surrounding text.
    ///
    /// This method is highly efficient and guarantees that strings are emitted
    /// in the exact sequential order of the original file, making it perfect
    /// for text replacement or masking.
    ///
    /// # Arguments
    /// * `window_size` - Size of the sliding window in bytes.
    /// * `overlap` - Overlap size in bytes (must be larger than the longest possible phone number).
    /// * `extract_matches` - Closure to extract numbers from a string slice.
    /// * `emit_phone` - Closure called with verified phone numbers.
    /// * `emit_non_phone` - Closure called with text chunks that are NOT phone numbers.
    fn search_phone_numbers<F, EP>(
        &self,
        window_size: usize,
        overlap: usize,
        mut extract_matches: F,
        mut emit_phone: EP,
    ) -> Result<(), super::SourceReadError>
    where
        F: FnMut(&str, &mut dyn FnMut(PhoneNumberMatch<'_>)),
        EP: FnMut(FoundToken),
    {
        let mut reader = self.read()?;
        let capacity = window_size.max(overlap * 2);
        let mut buffer = vec![0u8; capacity];
        let mut tail_len = 0;

        let mut absolute_offset = 0usize;
        let mut last_emitted_offset = 0usize;

        let mut active_matches: Vec<FoundNumber> = Vec::new();

        loop {
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
                break;
            }

            let (valid_text, valid_len) = match std::str::from_utf8(&buffer[..current_chunk_len]) {
                Ok(s) => (s, current_chunk_len),
                Err(e) => {
                    let len = e.valid_up_to();
                    (std::str::from_utf8(&buffer[..len]).unwrap(), len)
                }
            };

            let is_eof = bytes_read == 0;

            let mut yield_match = |candidate: PhoneNumberMatch| {
                let mut found = FoundNumber::from(candidate);
                found.start += absolute_offset;
                let mut overlap_indices = Vec::new();
                for (i, m) in active_matches.iter().enumerate() {
                    let max_start = found.start.max(m.start);
                    let min_end = (found.start + found.len).min(m.start + m.len);
                    if max_start < min_end {
                        overlap_indices.push(i);
                    }
                }

                if overlap_indices.is_empty() {
                    active_matches.push(found);
                } else {
                    let mut max_existing_len = 0;
                    for &i in &overlap_indices {
                        max_existing_len = max_existing_len.max(active_matches[i].len);
                    }

                    if found.len > max_existing_len {
                        for &i in overlap_indices.iter().rev() {
                            active_matches.remove(i);
                        }
                        active_matches.push(found);
                    }
                }
            };

            extract_matches(valid_text, &mut yield_match);
            let actual_start = if is_eof {
                valid_len
            } else {
                let mut start = valid_len.saturating_sub(overlap);
                while start < valid_len && !valid_text.is_char_boundary(start) {
                    start += 1;
                }
                start
            };

            let next_absolute_offset = absolute_offset + actual_start;
            active_matches.sort_by_key(|m| m.start);

            let mut i = 0;
            while i < active_matches.len() {
                if is_eof || active_matches[i].start + active_matches[i].len <= next_absolute_offset
                {
                    let m = active_matches.remove(i);
                    let m_start = m.start;
                    let m_len = m.len;
                    if last_emitted_offset < m_start {
                        let start_idx = last_emitted_offset.saturating_sub(absolute_offset);
                        let end_idx = m_start.saturating_sub(absolute_offset);

                        if start_idx < valid_len && end_idx <= valid_len && start_idx < end_idx {
                            emit_phone(FoundToken::NoPhone(&valid_text[start_idx..end_idx]));
                        }
                    }

                    last_emitted_offset = last_emitted_offset.max(m_start + m_len);
                    emit_phone(FoundToken::Phone(m));
                } else {
                    i += 1;
                }
            }

            let safe_text_end = if is_eof {
                absolute_offset + valid_len
            } else if let Some(first_active) = active_matches.first() {
                next_absolute_offset.min(first_active.start)
            } else {
                next_absolute_offset
            };

            if last_emitted_offset < safe_text_end {
                let start_idx = last_emitted_offset.saturating_sub(absolute_offset);
                let end_idx = safe_text_end.saturating_sub(absolute_offset);

                if start_idx < valid_len && end_idx <= valid_len && start_idx < end_idx {
                    emit_phone(FoundToken::NoPhone(&valid_text[start_idx..end_idx]));
                }
                last_emitted_offset = safe_text_end;
            }

            if is_eof {
                break;
            }

            let keep_len = valid_len - actual_start;
            buffer.copy_within(actual_start..valid_len, 0);

            let leftover_bytes = current_chunk_len - valid_len;
            if leftover_bytes > 0 {
                buffer.copy_within(valid_len..current_chunk_len, keep_len);
            }

            tail_len = keep_len + leftover_bytes;
            absolute_offset = next_absolute_offset;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sources::{FoundToken, ReadSource, SourceReadError};

    use super::*;
    use rlibphonenumber::phonenumber_matcher::FindNumberExt;
    use std::io::{BufRead, Cursor};
    #[derive(Debug, PartialEq)]
    enum Token {
        Text(String),
        Phone(String),
    }
    fn run_streaming_search(text: &str, window_size: usize, overlap: usize) -> Vec<Token> {
        struct MockSource {
            data: Vec<u8>,
        }
        impl ReadSource for MockSource {
            fn read(&self) -> Result<Box<dyn BufRead>, SourceReadError> {
                Ok(Box::new(Cursor::new(self.data.clone())))
            }
        }

        let source = MockSource {
            data: text.as_bytes().to_vec(),
        };
        let mut tokens = Vec::new();

        let extract = |valid_text: &str, yield_match: &mut dyn FnMut(PhoneNumberMatch<'_>)| {
            let matcher = valid_text.find_phone_numbers();
            for m in matcher {
                yield_match(m);
            }
        };

        let mut emit_phone = |found: FoundToken<'_>| {
            match found {
                FoundToken::Phone(found) => {
                    let raw_str = &text[found.start..found.start + found.len];
                    tokens.push(Token::Phone(raw_str.to_string()))
                }
                FoundToken::NoPhone(text) => tokens.push(Token::Text(text.to_string())),
            };
        };

        source
            .search_phone_numbers(window_size, overlap, extract, &mut emit_phone)
            .unwrap();

        tokens
    }

    #[test]
    fn test_empty_source() {
        let tokens = run_streaming_search("", 1024, 256);
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_only_text_no_phones() {
        let text = "Just some regular text without any numbers.";
        let tokens = run_streaming_search(text, 10, 5);
        assert_eq!(
            tokens,
            vec![
                Token::Text("Just ".to_string()),
                Token::Text("some ".to_string()),
                Token::Text("regul".to_string()),
                Token::Text("ar te".to_string()),
                Token::Text("xt wi".to_string()),
                Token::Text("thout".to_string()),
                Token::Text(" any ".to_string()),
                Token::Text("num".to_string()),
                Token::Text("bers.".to_string())
            ]
        );
    }

    #[test]
    fn test_phone_at_exact_boundaries() {
        let text = "+14155552671 text +14155552672";
        let tokens = run_streaming_search(text, 1024, 256);
        assert_eq!(
            tokens,
            vec![
                Token::Phone("+14155552671".to_string()),
                Token::Text(" text ".to_string()),
                Token::Phone("+14155552672".to_string()),
            ]
        );
    }

    #[test]
    fn test_consecutive_phones() {
        let text = "+14155552671+14155552672";
        let tokens = run_streaming_search(text, 1024, 256);
        assert_eq!(
            tokens,
            vec![
                Token::Phone("+14155552671".to_string()),
                Token::Phone("+14155552672".to_string()),
            ]
        );
    }

    #[test]
    fn test_utf8_boundaries() {
        let text = "Привет  🦀! Звони: +14155552671 Давай.";
        let tokens = run_streaming_search(text, 8, 8);

        assert_eq!(
            tokens,
            vec![
                Token::Text("При".to_string()),
                Token::Text("вет  ".to_string()),
                Token::Text("🦀! З".to_string()),
                Token::Text("вони".to_string()),
                Token::Text(": ".to_string()),
                Token::Phone("+14155552671".to_string()),
                Token::Text(" ".to_string()),
                Token::Text("Да".to_string()),
                Token::Text("вай.".to_string()),
            ]
        );
    }

    #[test]
    fn test_split_phone_number_overlap_reconstruction() {
        let text = "call +14155552671 now";
        let tokens = run_streaming_search(text, 12, 10);

        assert_eq!(
            tokens,
            vec![
                Token::Text("call ".to_string()),
                Token::Phone("+14155552671".to_string()),
                Token::Text(" now".to_string()),
            ]
        );
    }

    #[test]
    fn test_truncated_overlapping_matches() {
        let text = "test +1 415 555-2671 ext 1234 text";
        let tokens = run_streaming_search(text, 23, 20);

        assert_eq!(
            tokens,
            vec![
                Token::Text("test ".to_string()),
                Token::Phone("+1 415 555-2671 ext 1234".to_string()),
                Token::Text(" text".to_string()),
            ]
        );
    }
}
