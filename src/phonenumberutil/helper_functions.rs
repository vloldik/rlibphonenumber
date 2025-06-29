use std::collections::{HashMap, HashSet};

use protobuf::Message;
use strum::IntoEnumIterator;

use crate::{
    interfaces::MatcherApi,
    proto_gen::{
        phonemetadata::{PhoneMetadata, PhoneMetadataCollection, PhoneNumberDesc},
        phonenumber::PhoneNumber,
    },
};

use super::{
    PhoneNumberFormat, PhoneNumberType, ValidNumberLenType, ValidationResultErr,
    helper_constants::{
        METADATA, OPTIONAL_EXT_SUFFIX, PLUS_SIGN, POSSIBLE_CHARS_AFTER_EXT_LABEL,
        POSSIBLE_SEPARATORS_BETWEEN_NUMBER_AND_EXT_LABEL, RFC3966_EXTN_PREFIX, RFC3966_PREFIX,
    },
};

/// Loads metadata from helper constants METADATA array
pub(super) fn load_compiled_metadata() -> Result<PhoneMetadataCollection, protobuf::Error> {
    let result = PhoneMetadataCollection::parse_from_bytes(&METADATA)?;
    Ok(result)
}

/// Returns a pointer to the description inside the metadata of the appropriate
/// type.
pub(super) fn get_number_desc_by_type(
    metadata: &PhoneMetadata,
    phone_number_type: PhoneNumberType,
) -> &PhoneNumberDesc {
    match phone_number_type {
        PhoneNumberType::PremiumRate => &metadata.premium_rate,
        PhoneNumberType::TollFree => &metadata.toll_free,
        PhoneNumberType::Mobile => &metadata.mobile,
        PhoneNumberType::FixedLine | PhoneNumberType::FixedLineOrMobile => &metadata.fixed_line,
        PhoneNumberType::SharedCost => &metadata.shared_cost,
        PhoneNumberType::VoIP => &metadata.voip,
        PhoneNumberType::PersonalNumber => &metadata.personal_number,
        PhoneNumberType::Pager => &metadata.pager,
        PhoneNumberType::UAN => &metadata.uan,
        PhoneNumberType::VoiceMail => &metadata.voicemail,
        // Instead of the default case, we only match `Unknown`
        PhoneNumberType::Unknown => &metadata.general_desc,
    }
}

/// A helper function that is used by Format and FormatByPattern.
pub(super) fn prefix_number_with_country_calling_code(
    country_calling_code: i32,
    number_format: PhoneNumberFormat,
    formatted_number: &mut String,
) {
    if let PhoneNumberFormat::National = number_format {
        return;
    }
    let mut buf = itoa::Buffer::new();
    let country_calling_code_str = buf.format(country_calling_code);

    // we anyway allocate a new string in concatenation, so we'l do it once
    // with capacity of resulting string
    match number_format {
        PhoneNumberFormat::E164 => {
            let new_str =
                fast_cat::concat_str!(PLUS_SIGN, country_calling_code_str, &formatted_number);
            *formatted_number = new_str;
        }
        PhoneNumberFormat::International => {
            let new_str =
                fast_cat::concat_str!(PLUS_SIGN, country_calling_code_str, " ", &formatted_number);

            *formatted_number = new_str;
        }
        PhoneNumberFormat::RFC3966 => {
            let new_str = fast_cat::concat_str!(
                RFC3966_PREFIX,
                PLUS_SIGN,
                country_calling_code_str,
                "-",
                &formatted_number
            );

            *formatted_number = new_str;
        }
        // here code is already returned
        PhoneNumberFormat::National => {}
    }
}

// Returns true when one national number is the suffix of the other or both are
// the same.
pub(super) fn is_national_number_suffix_of_the_other(
    first_number: &PhoneNumber,
    second_number: &PhoneNumber,
) -> bool {
    let mut buf = itoa::Buffer::new();
    let first_number_national_number = buf.format(first_number.national_number());
    let mut buf = itoa::Buffer::new();
    let second_number_national_number = buf.format(second_number.national_number());
    // Note that HasSuffixString returns true if the numbers are equal.
    return first_number_national_number.ends_with(second_number_national_number)
        || second_number_national_number.ends_with(first_number_national_number);
}

/// Helper method for constructing regular expressions for parsing. Creates an
/// expression that captures up to max_length digits.
pub(super) fn extn_digits(max_length: u32) -> String {
    let mut buf = itoa::Buffer::new();
    let max_length_str = buf.format(max_length);
    const HELPER_STR_LEN: usize = 2 + 4 + 2;

    let mut expr = String::with_capacity(
        HELPER_STR_LEN + super::helper_constants::DIGITS.len() + max_length_str.len(),
    );

    expr.push_str("([");
    // Fully qualify DIGITS const as its common name
    expr.push_str(super::helper_constants::DIGITS);
    expr.push_str("]{1,");
    expr.push_str(max_length_str);
    expr.push_str("})");

    return expr;
}

// Helper initialiser method to create the regular-expression pattern to match
// extensions. Note that:
// - There are currently six capturing groups for the extension itself. If this
// number is changed, MaybeStripExtension needs to be updated.
// - The only capturing groups should be around the digits that you want to
// capture as part of the extension, or else parsing will fail!
pub(super) fn create_extn_pattern(for_parsing: bool) -> String {
    // We cap the maximum length of an extension based on the ambiguity of the
    // way the extension is prefixed. As per ITU, the officially allowed
    // length for extensions is actually 40, but we don't support this since we
    // haven't seen real examples and this introduces many false interpretations
    // as the extension labels are not standardized.
    let ext_limit_after_explicit_label = 20;
    let ext_limit_after_likely_label = 15;
    let ext_limit_after_ambiguous_char = 9;
    let ext_limit_when_not_sure = 6;

    // Canonical-equivalence doesn't seem to be an option with RE2, so we allow
    // two options for representing any non-ASCII character like ó - the character
    // itself, and one in the unicode decomposed form with the combining acute
    // accent.

    // Here the extension is called out in a more explicit way, i.e mentioning it
    // obvious patterns like "ext.".
    let explicit_ext_labels = "(?:e?xt(?:ensi(?:o\u{0301}?|\u{00F3}))?n?|(?:\u{FF45})?\u{FF58}\u{FF54}(?:\u{FF4E})?|\u{0434}\u{043E}\u{0431}|anexo)";
    // One-character symbols that can be used to indicate an extension, and less
    // commonly used or more ambiguous extension labels.
    let ambiguous_ext_labels = "(?:[x\u{FF58}#\u{FF03}~\u{FF5E}]|int|\u{FF49}\u{FF4E}\u{FF54})";
    // When extension is not separated clearly.
    let ambiguous_separator = "[- ]+";

    let rfc_extn = fast_cat::concat_str!(
        RFC3966_EXTN_PREFIX,
        &extn_digits(ext_limit_after_explicit_label)
    );
    let explicit_extn = fast_cat::concat_str!(
        POSSIBLE_SEPARATORS_BETWEEN_NUMBER_AND_EXT_LABEL,
        explicit_ext_labels,
        POSSIBLE_CHARS_AFTER_EXT_LABEL,
        &extn_digits(ext_limit_after_explicit_label),
        OPTIONAL_EXT_SUFFIX
    );
    let ambiguous_extn = fast_cat::concat_str!(
        POSSIBLE_SEPARATORS_BETWEEN_NUMBER_AND_EXT_LABEL,
        ambiguous_ext_labels,
        POSSIBLE_CHARS_AFTER_EXT_LABEL,
        &extn_digits(ext_limit_after_ambiguous_char),
        OPTIONAL_EXT_SUFFIX
    );

    let american_style_extn_with_suffix = fast_cat::concat_str!(
        ambiguous_separator,
        &extn_digits(ext_limit_when_not_sure),
        "#"
    );

    // The first regular expression covers RFC 3966 format, where the extension is
    // added using ";ext=". The second more generic where extension is mentioned
    // with explicit labels like "ext:". In both the above cases we allow more
    // numbers in extension than any other extension labels. The third one
    // captures when single character extension labels or less commonly used
    // labels are present. In such cases we capture fewer extension digits in
    // order to reduce the chance of falsely interpreting two numbers beside each
    // other as a number + extension. The fourth one covers the special case of
    // American numbers where the extension is written with a hash at the end,
    // such as "- 503#".
    let extension_pattern = fast_cat::concat_str!(
        &rfc_extn,
        "|",
        &explicit_extn,
        "|",
        &ambiguous_extn,
        "|",
        &american_style_extn_with_suffix
    );
    // Additional pattern that is supported when parsing extensions, not when
    // matching.
    if for_parsing {
        // ",," is commonly used for auto dialling the extension when connected.
        // Semi-colon works in Iphone and also in Android to pop up a button with
        // the extension number following.
        let auto_dialling_and_ext_labels_found = "(?:,{2}|;)";
        // This is same as kPossibleSeparatorsBetweenNumberAndExtLabel, but not
        // matching comma as extension label may have it.
        let possible_separators_number_ext_label_no_comma = "[ \u{00A0}\t]*";

        let auto_dialling_extn = fast_cat::concat_str!(
            possible_separators_number_ext_label_no_comma,
            auto_dialling_and_ext_labels_found,
            POSSIBLE_CHARS_AFTER_EXT_LABEL,
            &extn_digits(ext_limit_after_likely_label),
            OPTIONAL_EXT_SUFFIX
        );
        let only_commas_extn = fast_cat::concat_str!(
            possible_separators_number_ext_label_no_comma,
            "(?:,)+",
            POSSIBLE_CHARS_AFTER_EXT_LABEL,
            &extn_digits(ext_limit_after_ambiguous_char),
            OPTIONAL_EXT_SUFFIX
        );
        // Here the first pattern is exclusive for extension autodialling formats
        // which are used when dialling and in this case we accept longer
        // extensions. However, the second pattern is more liberal on number of
        // commas that acts as extension labels, so we have strict cap on number of
        // digits in such extensions.
        return fast_cat::concat_str!(
            &extension_pattern,
            "|",
            &auto_dialling_extn,
            "|",
            &only_commas_extn
        );
    }
    return extension_pattern;
}

/// Normalizes a string of characters representing a phone number by replacing
/// all characters found in the accompanying map with the values therein, and
/// stripping all other characters if remove_non_matches is true.
///
/// Parameters:
/// * `number` - a pointer to a string of characters representing a phone number to
///   be normalized.
/// * `normalization_replacements` - a mapping of characters to what they should be
///   replaced by in the normalized version of the phone number
/// * `remove_non_matches` - indicates whether characters that are not able to be
///   replaced should be stripped from the number. If this is false, they will be
///   left unchanged in the number.
pub(super) fn normalize_helper(
    normalization_replacements: &HashMap<char, char>,
    remove_non_matches: bool,
    phone_number: &mut String,
) {
    let mut normalized_number = String::with_capacity(phone_number.len());
    // Skip UTF checking because strings in rust are valid UTF-8 already
    for phone_char in phone_number.chars() {
        if let Some(replacement) = normalization_replacements.get(&phone_char.to_ascii_uppercase()) {
            normalized_number.push(*replacement);
        } else if !remove_non_matches {
            normalized_number.push(phone_char);
        }
        // If neither of the above are true, we remove this character.
    }

    *phone_number = normalized_number;
}

/// Returns `true` if there is any possible number data set for a particular
/// PhoneNumberDesc.
pub(super) fn desc_has_possible_number_data(desc: &PhoneNumberDesc) -> bool {
    // If this is empty, it means numbers of this type inherit from the "general
    // desc" -> the value "-1" means that no numbers exist for this type.
    return desc.possible_length.len() != 1
        || desc
            .possible_length
            .get(0)
            .and_then(|l| Some(*l != -1))
            .unwrap_or(false);
}

/// Note: `DescHasData` must account for any of MetadataFilter's
/// excludableChildFields potentially being absent from the metadata. It must
/// check them all. For any changes in `DescHasData`, ensure that all the
/// excludableChildFields are still being checked.
///
/// If your change is safe simply
/// mention why during a review without needing to change MetadataFilter.
///
/// Returns `true` if there is any data set for a particular PhoneNumberDesc.
pub(super) fn desc_has_data(desc: &PhoneNumberDesc) -> bool {
    // Checking most properties since we don't know what's present, since a custom
    // build may have stripped just one of them (e.g. USE_METADATA_LITE strips
    // exampleNumber). We don't bother checking the PossibleLengthsLocalOnly,
    // since if this is the only thing that's present we don't really support the
    // type at all: no type-specific methods will work with only this data.
    return desc.has_example_number()
        || desc_has_possible_number_data(desc)
        || desc.has_national_number_pattern();
}

/// Returns the types we have metadata for based on the PhoneMetadata object
/// passed in.
pub(super) fn get_supported_types_for_metadata(
    metadata: &PhoneMetadata,
    types: &mut HashSet<PhoneNumberType>,
) {
    PhoneNumberType::iter()
        // Never return FIXED_LINE_OR_MOBILE (it is a convenience type, and
        // represents that a particular number type can't be
        // determined) or UNKNOWN (the non-type).
        .filter(|number_type| {
            !matches!(
                number_type,
                PhoneNumberType::FixedLineOrMobile | PhoneNumberType::Unknown
            )
        })
        .filter(|number_type| desc_has_data(get_number_desc_by_type(metadata, *number_type)))
        .for_each(|number_type| {
            types.insert(number_type);
        });
}

/// Helper method to check a number against possible lengths for this number
/// type, and determine whether it matches, or is too short or too long.
pub(super) fn test_number_length(
    phone_number: &str,
    phone_metadata: &PhoneMetadata,
    phone_number_type: PhoneNumberType,
) -> Result<ValidNumberLenType, ValidationResultErr> {
    let desc_for_type = get_number_desc_by_type(phone_metadata, phone_number_type);
    // There should always be "possibleLengths" set for every element. This is
    // declared in the XML schema which is verified by
    // PhoneNumberMetadataSchemaTest. For size efficiency, where a
    // sub-description (e.g. fixed-line) has the same possibleLengths as the
    // parent, this is missing, so we fall back to the general desc (where no
    // numbers of the type exist at all, there is one possible length (-1) which
    // is guaranteed not to match the length of any real phone number).
    let mut possible_lengths = if desc_for_type.possible_length.len() == 0 {
        phone_metadata.general_desc.possible_length.clone()
    } else {
        desc_for_type.possible_length.clone()
    };

    let mut local_lengths = desc_for_type.possible_length_local_only.clone();
    if phone_number_type == PhoneNumberType::FixedLineOrMobile {
        let fixed_line_desc = get_number_desc_by_type(phone_metadata, PhoneNumberType::FixedLine);
        if !desc_has_possible_number_data(fixed_line_desc) {
            // The rare case has been encountered where no fixedLine data is available
            // (true for some non-geographical entities), so we just check mobile.
            return test_number_length(phone_number, phone_metadata, PhoneNumberType::Mobile);
        } else {
            let mobile_desc = get_number_desc_by_type(phone_metadata, PhoneNumberType::Mobile);
            if desc_has_possible_number_data(mobile_desc) {
                // Merge the mobile data in if there was any. Note that when adding the
                // possible lengths from mobile, we have to again check they aren't
                // empty since if they are this indicates they are the same as the
                // general desc and should be obtained from there.

                // RUST NOTE: since merge adds elements to the end of the list, we can do the same
                let len_to_append = if mobile_desc.possible_length.len() == 0 {
                    &phone_metadata.general_desc.possible_length
                } else {
                    &mobile_desc.possible_length
                };
                possible_lengths.extend_from_slice(len_to_append);
                possible_lengths.sort();

                if local_lengths.len() == 0 {
                    local_lengths = mobile_desc.possible_length_local_only.clone();
                } else {
                    local_lengths.extend_from_slice(&mobile_desc.possible_length_local_only);
                    local_lengths.sort();
                }
            }
        }
    }

    // If the type is not suported at all (indicated by the possible lengths
    // containing -1 at this point) we return invalid length.
    if *possible_lengths.first().unwrap_or(&-1) == -1 {
        return Err(ValidationResultErr::InvalidLength);
    }

    let actual_length = phone_number.len() as i32;
    // This is safe because there is never an overlap beween the possible lengths
    // and the local-only lengths; this is checked at build time.
    if local_lengths.contains(&actual_length) {
        return Ok(ValidNumberLenType::IsPossibleLocalOnly);
    }

    // here we can unwrap safe
    let minimum_length = possible_lengths[0];

    if minimum_length == actual_length {
        return Ok(ValidNumberLenType::IsPossible);
    } else if minimum_length > actual_length {
        return Err(ValidationResultErr::TooShort);
    } else if possible_lengths[possible_lengths.len() - 1] < actual_length {
        return Err(ValidationResultErr::TooLong);
    }
    // We skip the first element; we've already checked it.
    return if possible_lengths[1..].contains(&actual_length) {
        Ok(ValidNumberLenType::IsPossible)
    } else {
        Err(ValidationResultErr::InvalidLength)
    };
}

/// Helper method to check a number against possible lengths for this region,
/// based on the metadata being passed in, and determine whether it matches, or
/// is too short or too long.
pub(super) fn test_number_length_with_unknown_type(
    phone_number: &str,
    phone_metadata: &PhoneMetadata,
) -> Result<ValidNumberLenType, ValidationResultErr> {
    return test_number_length(phone_number, phone_metadata, PhoneNumberType::Unknown);
}

/// Returns a new phone number containing only the fields needed to uniquely
/// identify a phone number, rather than any fields that capture the context in
/// which the phone number was created.
/// These fields correspond to those set in `parse()` rather than
/// `parse_and_keep_raw_input()`.
pub(crate) fn copy_core_fields_only(from_number: &PhoneNumber, to_number: &mut PhoneNumber) {
    to_number.set_country_code(from_number.country_code());
    to_number.set_national_number(from_number.national_number());
    if let Some(extension) = &from_number.extension {
        to_number.set_extension(extension.clone());
    }
    if from_number.italian_leading_zero() {
        to_number.set_italian_leading_zero(true);
        // This field is only relevant if there are leading zeros at all.
        to_number.set_number_of_leading_zeros(from_number.number_of_leading_zeros());
    }
}

/// Determines whether the given number is a national number match for the given
/// PhoneNumberDesc. Does not check against possible lengths!
pub(super) fn is_match(
    matcher_api: Box<dyn MatcherApi>,
    number: &str,
    number_desc: &PhoneNumberDesc,
) -> bool {
    matcher_api.match_national_number(number, number_desc, false)
}
