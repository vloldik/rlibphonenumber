use crate::phonenumberutil::regex_wrapper_types::{PhoneMetadataWrapper, RegexTriplets};
use crate::{PhoneMetadataCollection, Region};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetadataValidationError {
    #[error("Invalid region code found in metadata: '{region_id}'")]
    InvalidRegionCode { region_id: String },

    #[error("Regex compilation failed for {context}: {error}")]
    RegexCompilationFailed { context: String, error: String },

    #[error(
        "Regex pattern base does not start with '^(?:' and end with ')$' in {context}. Base was: '{base}'"
    )]
    InvalidRegexWrapping { context: String, base: String },

    #[error(
        "Invalid length found in {context}: Length {length} >= 64. Metadata MUST have at most 63 as a possible length."
    )]
    LengthTooLarge { context: String, length: i32 },

    #[error("Invalid length found in {context}: Length {length} cannot be negative.")]
    NegativeLength { context: String, length: i32 },
}

/// Validates a given `PhoneMetadataCollection`: checks regexps, lengths and regions.
pub fn validate_metadata(
    collection: PhoneMetadataCollection,
    alternate_formats: bool,
) -> Result<(), MetadataValidationError> {
    for metadata in collection.metadata.into_iter() {
        let region_id = metadata.id.clone();

        if !alternate_formats && Region::from_code(&region_id).is_err() {
            return Err(MetadataValidationError::InvalidRegionCode { region_id });
        }

        let wrapper = PhoneMetadataWrapper::from(metadata);
        test_triplets(
            wrapper.international_prefix(),
            &format!("{} - international_prefix", region_id),
        )?;
        test_triplets(
            wrapper.leading_digits(),
            &format!("{} - leading_digits", region_id),
        )?;
        test_triplets(
            wrapper.national_prefix_for_parsing(),
            &format!("{} - national_prefix_for_parsing", region_id),
        )?;

        for (i, fmt) in wrapper.number_format.iter().enumerate() {
            test_triplets(
                fmt.pattern(),
                &format!("{} - number_format[{}] pattern", region_id, i),
            )?;
            for (j, ld) in fmt.leading_digits_pattern().iter().enumerate() {
                test_triplets(
                    ld,
                    &format!("{} - number_format[{}].leading_digits[{}]", region_id, i, j),
                )?;
            }
        }

        for (i, fmt) in wrapper.intl_number_format.iter().enumerate() {
            test_triplets(
                fmt.pattern(),
                &format!("{} - intl_number_format[{}] pattern", region_id, i),
            )?;
            for (j, ld) in fmt.leading_digits_pattern().iter().enumerate() {
                test_triplets(
                    ld,
                    &format!(
                        "{} - intl_number_format[{}].leading_digits[{}]",
                        region_id, i, j
                    ),
                )?;
            }
        }

        macro_rules! check_desc {
            ($name:ident) => {{
                let ctx = format!("{} - {}", region_id, stringify!($name));
                test_triplets(wrapper.$name.national_number_pattern(), &ctx)?;
                let original_desc = &wrapper.$name.original;
                check_lengths(
                    &original_desc.possible_length,
                    &format!("{}.possible_length", ctx),
                )?;
                check_lengths(
                    &original_desc.possible_length_local_only,
                    &format!("{}.possible_length_local_only", ctx),
                )?;
            }};
        }

        check_desc!(general_desc);
        check_desc!(fixed_line);
        check_desc!(mobile);
        check_desc!(toll_free);
        check_desc!(premium_rate);
        check_desc!(shared_cost);
        check_desc!(personal_number);
        check_desc!(voip);
        check_desc!(pager);
        check_desc!(uan);
        check_desc!(emergency);
        check_desc!(voicemail);
        check_desc!(short_code);
        check_desc!(standard_rate);
        check_desc!(carrier_specific);
        check_desc!(sms_services);
        check_desc!(no_international_dialling);
    }

    Ok(())
}

fn test_triplets(triplets: &RegexTriplets, context: &str) -> Result<(), MetadataValidationError> {
    triplets
        .anchor_full()
        .map_err(|e| MetadataValidationError::RegexCompilationFailed {
            context: format!("{}: anchor_full", context),
            error: e.to_string(),
        })?;

    triplets
        .anchor_start()
        .map_err(|e| MetadataValidationError::RegexCompilationFailed {
            context: format!("{}: anchor_start", context),
            error: e.to_string(),
        })?;

    triplets
        .original()
        .map_err(|e| MetadataValidationError::RegexCompilationFailed {
            context: format!("{}: original", context),
            error: e.to_string(),
        })?;

    triplets.original_base();

    if let Some(base) = &triplets.pattern_base
        && !base.is_empty()
        && !(base.starts_with("^(?:") && base.ends_with(")$"))
    {
        return Err(MetadataValidationError::InvalidRegexWrapping {
            context: context.to_string(),
            base: base.clone(),
        });
    }

    Ok(())
}

fn check_lengths(lengths: &[i32], context: &str) -> Result<(), MetadataValidationError> {
    for &length in lengths {
        if length >= 64 {
            return Err(MetadataValidationError::LengthTooLarge {
                context: context.to_string(),
                length,
            });
        }
        if length < 0 && length != -1 {
            return Err(MetadataValidationError::NegativeLength {
                context: context.to_string(),
                length,
            });
        }
    }
    Ok(())
}
