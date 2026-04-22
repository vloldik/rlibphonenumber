use rustc_hash::FxHashMap;

use crate::{PhoneMetadataCollection, phonenumberutil::regex_wrapper_types::PhoneMetadataWrapper};

pub struct AlternateFormats {
    calling_code_to_alternate_formats_map: FxHashMap<i32, PhoneMetadataWrapper>,
}

impl AlternateFormats {
    pub fn new(metadata_collection: PhoneMetadataCollection) -> Self {
        let mut calling_code_to_alternate_formats_map = FxHashMap::default();
        for metadata in metadata_collection.metadata {
            calling_code_to_alternate_formats_map.insert(metadata.country_code(), metadata.into());
        }
        Self {
            calling_code_to_alternate_formats_map,
        }
    }

    pub fn get_alternate_formats_for_country(
        &self,
        country_calling_code: i32,
    ) -> Option<&PhoneMetadataWrapper> {
        self.calling_code_to_alternate_formats_map
            .get(&country_calling_code)
    }
}
