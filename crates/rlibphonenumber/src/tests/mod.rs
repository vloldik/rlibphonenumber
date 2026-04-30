mod fuzz_artifacts;
mod matcher_tests;
mod metadata_tests;
mod phonenumberutil_tests;

mod common {
    use std::{env, sync::LazyLock};

    use prost::Message;

    use crate::{
        PhoneMetadataCollection, generated::metadata::TEST_METADATA,
        phonenumber_matcher::PhoneNumberMatcherFactory,
        phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
    };

    pub static ONCE: std::sync::Once = std::sync::Once::new();
    static UTIL: LazyLock<PhoneNumberUtilInternal> =
        LazyLock::new(|| PhoneNumberUtilInternal::new_for_metadata(load_metadata()).unwrap());

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
    -> PhoneNumberMatcherFactory<PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal> {
        PhoneNumberMatcherFactory::new_for_util(get_phone_util())
    }

    pub fn load_metadata() -> PhoneMetadataCollection {
        PhoneMetadataCollection::decode(TEST_METADATA).expect("Metadata should be valid")
    }
}
