use prost::Message;

use crate::{
    PhoneMetadataCollection,
    generated::metadata::{ALTERNATE_FORMATS_METADATA, METADATA},
    metadata_validator::validate_metadata,
};

#[test]
#[allow(deprecated)]
fn test_metadata_regexps() {
    validate_metadata(
        PhoneMetadataCollection::decode(METADATA).expect("Metadata must parse"),
        false,
    )
    .expect("Metadata should be valid");

    validate_metadata(
        PhoneMetadataCollection::decode(ALTERNATE_FORMATS_METADATA).expect("Metadata must parse"),
        true,
    )
    .expect("Metadata should be valid");
}
