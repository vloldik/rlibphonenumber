mod fuzz_artifacts;
mod matcher_tests;
mod metadata_tests;
mod phonenumberutil_tests;

mod common {
    use std::sync::Arc;

    use prost::Message;

    use crate::{
        PhoneMetadataCollection, generated::metadata::TEST_METADATA,
        phonenumber_matcher::PhoneNumberMatcherFactory,
        phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
    };

    pub static ONCE: std::sync::Once = std::sync::Once::new();

    pub fn get_phone_util() -> PhoneNumberUtilInternal {
        ONCE.call_once(|| {
            colog::default_builder()
                .filter_level(log::LevelFilter::Trace)
                .init()
        });

        let metadata = load_metadata();
        PhoneNumberUtilInternal::new_for_metadata(metadata).unwrap()
    }

    pub fn get_phone_matcher_factory()
    -> PhoneNumberMatcherFactory<PhoneNumberUtilInternal, Arc<PhoneNumberUtilInternal>> {
        PhoneNumberMatcherFactory::new(
            Arc::new(PhoneNumberUtilInternal::new_for_metadata(load_metadata()).unwrap()),
            None,
        )
    }

    pub fn load_metadata() -> PhoneMetadataCollection {
        PhoneMetadataCollection::decode(TEST_METADATA).expect("Metadata should be valid")
    }
}
