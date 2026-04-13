#[cfg(test)]
mod fuzz_artifacts;
#[cfg(test)]
mod metadata_tests;
#[cfg(test)]
mod phonenumberutil_tests;
#[cfg(test)]
mod region_code;

#[cfg(test)]
mod common {
    use crate::{
        PhoneMetadataCollection, generated::metadata::TEST_METADATA,
        phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
    };

    pub static ONCE: std::sync::Once = std::sync::Once::new();

    pub const UNKNOWN_REGION_CODE: Option<&str> = Some("ZZ");

    #[cfg(test)]
    pub fn get_phone_util() -> PhoneNumberUtilInternal {
        use prost::{Message, bytes::Bytes};

        ONCE.call_once(|| {
            colog::default_builder()
                .filter_level(log::LevelFilter::Trace)
                .init()
        });

        let metadata = PhoneMetadataCollection::decode(Bytes::from_static(&TEST_METADATA))
            .expect("Metadata should be valid");
        PhoneNumberUtilInternal::new_for_metadata(metadata)
    }
}
