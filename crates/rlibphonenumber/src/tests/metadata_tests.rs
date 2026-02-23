use crate::{
    phonemetadata::PhoneMetadataCollection,
    phonenumberutil::regex_wrapper_types::{PhoneMetadataWrapper, RegexTriplets},
};

fn load_metadata() -> PhoneMetadataCollection {
    use protobuf::Message;

    use crate::generated::metadata::METADATA;

    PhoneMetadataCollection::parse_from_bytes(&METADATA).expect("Metadata should be valid")
}

fn test_triplets(triplets: &RegexTriplets) {
    triplets.anchor_full().unwrap();
    triplets.anchor_start().unwrap();
    triplets.original().unwrap();
    triplets.original_base();
    assert!(
        triplets
            .pattern_base
            .as_ref()
            .take_if(|base| !base.is_empty())
            .is_none_or(|base| base.starts_with("^(?:") && base.ends_with(")$"))
    );
}

#[test]
#[allow(deprecated)]
fn test_metadata_regexps() {
    load_metadata().metadata.into_iter().for_each(|metadata| {
        let w = PhoneMetadataWrapper::from(metadata);

        test_triplets(w.international_prefix());
        test_triplets(w.leading_digits());
        test_triplets(w.national_prefix_for_parsing());
        w.number_format.into_iter().for_each(|w| {
            test_triplets(w.pattern());
            w.leading_digits_pattern().iter().for_each(test_triplets);
        });

        w.intl_number_format.into_iter().for_each(|w| {
            test_triplets(w.pattern());
            w.leading_digits_pattern().iter().for_each(test_triplets);
        });

        macro_rules! test_desc_wrapper {
            ($($name:ident),*) => {
                $(test_triplets(w.$name.national_number_pattern());)*
            };
        }

        test_desc_wrapper!(
            general_desc,
            fixed_line,
            mobile,
            toll_free,
            premium_rate,
            shared_cost,
            personal_number,
            voip,
            pager,
            uan,
            emergency,
            voicemail,
            short_code,
            standard_rate,
            carrier_specific,
            sms_services,
            no_international_dialling
        );
    });
}
