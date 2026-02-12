#[cfg(test)]
use crate::phonemetadata::PhoneMetadataCollection;
#[cfg(test)]
use crate::phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal;

#[cfg(test)]
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

#[test]
fn test_valid_metadata_for_valid_region() {
    let util = PhoneNumberUtilInternal::new_for_metadata(load_metadata());
    for first_char in 'A'..='Z' {
        for second_char in 'A'..='Z' {
            let region = format!("{}{}", first_char, second_char);
            let Some(code) = util.get_country_code_for_region(&region) else {
                continue;
            };
            assert!(util.has_valid_country_calling_code(code));
            util.get_metadata_for_region_or_calling_code(code, &region)
                .expect("Metadata must exist for valid country calling code");
        }
    }
}
