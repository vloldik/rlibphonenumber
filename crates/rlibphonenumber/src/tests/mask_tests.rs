#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hash, Hasher};

    use crate::{
        Region,
        interfaces::{LenWrite, PhoneHasher},
        phonenumber_mask::{Hashed, MaskDigitsConfig, MaskType, PhoneStdHasher, mask_number},
        tests::common::get_phone_util,
    };

    struct AllocationTrackedWriter {
        inner: String,
        grow_called: bool,
    }

    impl AllocationTrackedWriter {
        fn new() -> Self {
            Self {
                inner: String::new(),
                grow_called: false,
            }
        }

        fn into_inner(self) -> String {
            self.inner
        }
    }

    impl LenWrite for AllocationTrackedWriter {
        fn grow(&mut self, len: usize) {
            assert!(!self.grow_called, "grow() called twice!");
            self.inner.reserve_exact(len);
            self.grow_called = true;
        }

        fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
            let cap_before = self.inner.capacity();
            let s = std::str::from_utf8(buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            self.inner.push_str(s);

            if self.grow_called {
                let cap_after = self.inner.capacity();
                assert_eq!(
                    cap_before, cap_after,
                    "Unexpected Alloc! grow() did not reserve enough. Capacity changed from {} to {}",
                    cap_before, cap_after
                );
            }

            Ok(())
        }
    }

    #[test]
    fn normal_local_keep_last() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "123-4567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "***-4567");
    }

    #[test]
    fn normal_intl_keep_last() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "+1 (918) 123-4567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "+* (***) ***-4567");
    }

    #[test]
    fn normal_intl_mask_all() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::new('*', false));
        let raw_input = "+1 918 123-4567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "+* *** ***-****");
    }

    #[test]
    fn edge_short_number() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "911";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "***");
    }

    #[test]
    fn edge_exactly_seven_digits() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "1234567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "***4567");
    }

    #[test]
    fn edge_no_digits() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "+1-123-FLOWERS";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        // Letters are masked because they map to numbers
        assert_eq!(writer.into_inner(), "+*-***-***WERS");
    }

    #[test]
    fn format_preserve_ext_and_context() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "tel:863-1234;phone-context=+1-918;ext=123";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        // Context and extension must be masked entirely
        assert_eq!(
            writer.into_inner(),
            "tel:***-1234;phone-context=******;ext=***"
        );
    }

    #[test]
    fn unicode_cyrillic_mask() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::new('Ж', true));
        let raw_input = "123-4567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "ЖЖЖ-4567");
    }

    #[test]
    fn unicode_emoji_mask() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::new('🔒', true));
        let raw_input = "+1 918 123-4567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "+🔒 🔒🔒🔒 🔒🔒🔒-4567");
    }

    #[test]
    fn fixed_mask() {
        let mask = MaskType::Fixed("REDACTED".into());
        let raw_input = "+19181234567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "REDACTED");
    }

    #[test]
    fn semantic_token_no_hash() {
        let mask = MaskType::SemanticToken {
            region: Some(Region::US),
            add_hash: None,
        };
        let raw_input = "+19181234567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "<Phone country=\"US\">");
    }

    #[test]
    fn semantic_token_world_fallback() {
        let mask = MaskType::SemanticToken {
            region: None,
            add_hash: None,
        };
        let raw_input = "+80012345678";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "<Phone country=\"001\">");
    }

    #[test]
    fn semantic_token_with_hash() {
        let fake_hash = Hashed::from_slice(b"fakehash").unwrap();
        let mask = MaskType::SemanticToken {
            region: Some(Region::US),
            add_hash: Some(fake_hash),
        };
        let raw_input = "+19181234567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");

        let result = writer.into_inner();
        assert!(result.starts_with("<Phone country=\"US\" hash=\""));
        assert!(result.ends_with("\">"));
        assert_eq!(result, "<Phone country=\"US\" hash=\"66616b6568617368\">");
    }

    #[test]
    fn hash_only_mask() {
        let fake_hash = Hashed::from_slice(&[0x12, 0x34, 0xab, 0xcd]).unwrap();
        let mask = MaskType::Hash(fake_hash);
        let raw_input = "+19181234567";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        let result = writer.into_inner();
        assert_eq!(result, "1234abcd");
    }

    #[test]
    fn hashed_from_slice_validations() {
        let max_slice = [0xff; 64];
        let hash_max = Hashed::from_slice(&max_slice).unwrap();
        assert_eq!(hash_max.as_slice(), max_slice);

        let min_slice = [0x00; 0];
        let hash_min = Hashed::from_slice(&min_slice).unwrap();
        assert_eq!(hash_min.as_slice(), &[]);

        let mid_slice = b"hello_world";
        let hash_mid = Hashed::from_slice(mid_slice).unwrap();
        assert_eq!(hash_mid.as_slice(), mid_slice);
    }

    #[test]
    fn hashed_from_slice_none_on_too_large() {
        let too_big_slice = [0x00; 65];
        assert_eq!(Hashed::from_slice(&too_big_slice), None);
    }

    #[test]
    fn hashed_eq_and_hash_traits() {
        let h1 = Hashed::from_slice(b"same_data");
        let h2 = Hashed::from_slice(b"same_data");
        let h3 = Hashed::from_slice(b"diff_data");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        h1.hash(&mut hasher1);
        h2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn std_hasher_integration() {
        let util = get_phone_util();
        let tel = util.parse("+19181234567", None).unwrap();
        let hasher = DefaultHasher::new();
        let hashed_result = PhoneStdHasher(hasher).hash_phone(&tel);

        assert!(hashed_result.is_some());
        let hashed = hashed_result.unwrap();
        assert_eq!(hashed.as_slice().len(), 8);
    }

    #[test]
    fn edge_empty_input() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "");
    }

    #[test]
    fn edge_only_context_no_digits_in_main() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "tel:;phone-context=+1234";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "tel:;phone-context=*****");
    }

    #[test]
    fn edge_all_letters_no_real_digits() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "CALL-ME-NOW";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");
        assert_eq!(writer.into_inner(), "****-*E-NOW");
    }

    #[test]
    fn edge_extension_only() {
        let mask = MaskType::MaskDigits(MaskDigitsConfig::default());
        let raw_input = "доб. 12345";
        let util = get_phone_util();
        let mut writer = AllocationTrackedWriter::new();
        mask_number(util, raw_input, &mask, &mut writer).expect("Failed to mask");

        assert_eq!(writer.into_inner(), "доб. *****");
    }

    #[test]
    fn debug_representation() {
        let h = Hashed::from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        let debug_str = format!("{:?}", h);
        assert_eq!(debug_str, "Some(Hash(\"deadbeef\"))");
    }
}
