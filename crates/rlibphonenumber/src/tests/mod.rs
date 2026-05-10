mod mask_tests;
mod matcher_tests;
mod metadata_tests;
mod phonenumberutil_tests;

mod common {
    use std::{env, sync::LazyLock};

    use prost::Message;

    use crate::{
        PhoneMetadataCollection, generated::metadata::TEST_METADATA,
        phonenumber_mask::PhoneMaskUtil, phonenumber_matcher::PhoneNumberMatcherFactory,
        phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
    };

    pub static ONCE: std::sync::Once = std::sync::Once::new();
    static UTIL: LazyLock<PhoneNumberUtilInternal> =
        LazyLock::new(|| PhoneNumberUtilInternal::new_for_metadata(load_metadata()).unwrap());
    static MATCHER_FACTORY: LazyLock<
        PhoneNumberMatcherFactory<PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal>,
    > = LazyLock::new(|| PhoneNumberMatcherFactory::new_for_util(get_phone_util()));

    pub fn get_phone_mask_util()
    -> PhoneMaskUtil<PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal> {
        PhoneMaskUtil::new_for_util(get_phone_util())
    }

    pub fn get_phone_util() -> &'static PhoneNumberUtilInternal {
        ONCE.call_once(|| {
            if env::var("RUST_LOG").is_ok_and(|var| var.to_lowercase() == "trace") {
                colog::default_builder()
                    .filter_level(log::LevelFilter::Trace)
                    .init()
            }
        });

        &UTIL
    }

    pub fn get_phone_matcher_factory()
    -> &'static PhoneNumberMatcherFactory<PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal>
    {
        &MATCHER_FACTORY
    }

    pub fn load_metadata() -> PhoneMetadataCollection {
        PhoneMetadataCollection::decode(TEST_METADATA).expect("Metadata should be valid")
    }
}
