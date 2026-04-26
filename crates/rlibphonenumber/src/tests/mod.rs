#[cfg(test)]
mod fuzz_artifacts;
#[cfg(test)]
mod metadata_tests;
#[cfg(test)]
mod phonenumberutil_tests;

#[cfg(test)]
mod common {
    use prost::Message;

    use crate::{
        PhoneMetadataCollection, generated::metadata::TEST_METADATA,
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

    pub fn load_metadata() -> PhoneMetadataCollection {
        PhoneMetadataCollection::decode(TEST_METADATA).expect("Metadata should be valid")
    }
}
