use crate::phonemetadata::PhoneMetadataCollection;

fn load_metadata() -> PhoneMetadataCollection {
    use protobuf::Message;

    use crate::generated::metadata::METADATA;

    PhoneMetadataCollection::parse_from_bytes(&METADATA).expect("Metadata should be valid")
}

#[test]
fn test_metadata_regexps() {
    use regex::Regex;

    load_metadata().metadata.iter().for_each(|metadata| {
        metadata.number_format.iter().for_each(|f| {
            if f.has_pattern() {
                Regex::new(f.pattern()).expect("Regexp MUST be valid");
            }
        });

        metadata.intl_number_format.iter().for_each(|f| {
            if f.has_pattern() {
                Regex::new(f.pattern()).expect("Regexp MUST be valid");
            }
        });
    });
}
