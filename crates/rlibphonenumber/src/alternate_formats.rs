use rustc_hash::FxHashMap;

use crate::{PhoneMetadataCollection, phonenumberutil::regex_wrapper_types::PhoneMetadataWrapper};

#[cfg(feature = "builtin_metadata")]
use crate::generated::metadata::ALTERNATE_FORMATS_METADATA;
#[cfg(feature = "builtin_metadata")]
use prost::{DecodeError, Message};

#[derive(Debug)]
pub struct AlternateFormats {
    calling_code_to_alternate_formats_map: FxHashMap<i32, PhoneMetadataWrapper>,
}

impl AlternateFormats {
    pub fn new_for_metadata(metadata_collection: PhoneMetadataCollection) -> Self {
        let mut calling_code_to_alternate_formats_map = FxHashMap::default();
        for metadata in metadata_collection.metadata {
            calling_code_to_alternate_formats_map.insert(metadata.country_code(), metadata.into());
        }
        Self {
            calling_code_to_alternate_formats_map,
        }
    }

    #[cfg(feature = "builtin_metadata")]
    pub fn try_new() -> Result<Self, DecodeError> {
        Ok(Self::new_for_metadata(PhoneMetadataCollection::decode(
            ALTERNATE_FORMATS_METADATA,
        )?))
    }

    #[cfg(feature = "builtin_metadata")]
    pub fn new() -> Self {
        Self::try_new().unwrap()
    }

    pub fn get_alternate_formats_for_country(
        &self,
        country_calling_code: i32,
    ) -> Option<&PhoneMetadataWrapper> {
        self.calling_code_to_alternate_formats_map
            .get(&country_calling_code)
    }
}
