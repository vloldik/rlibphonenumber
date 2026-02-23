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
        generated::metadata::TEST_METADATA, phonemetadata::PhoneMetadataCollection,
        phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
    };

    pub static ONCE: std::sync::Once = std::sync::Once::new();

    pub const UNKNOWN_REGION_CODE: Option<&str> = Some("ZZ");

    #[cfg(test)]
    pub fn get_phone_util() -> PhoneNumberUtilInternal {
        use protobuf::Message;

        ONCE.call_once(|| {
            colog::default_builder()
                .filter_level(log::LevelFilter::Trace)
                .init()
        });

        let metadata = PhoneMetadataCollection::parse_from_bytes(&TEST_METADATA)
            .expect("Metadata should be valid");
        PhoneNumberUtilInternal::new_for_metadata(metadata)
    }
}
