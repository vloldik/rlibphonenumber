use std::{borrow::Cow, fmt::Display, str::FromStr};

use crate::{
    NumberLengthType, PHONE_NUMBER_UTIL, ParseError, PhoneNumber, PhoneNumberFormat,
    PhoneNumberType, ValidationError, enums::Region,
    phonenumberutil::helper_functions::get_national_significant_number_owned,
};

impl PhoneNumber {
    pub(crate) fn number_of_leading_zeros_safe_for_formatting(&self) -> usize {
        self.number_of_leading_zeros()
            .try_into()
            .unwrap_or(0)
            .min(10)
    }

    pub fn format_as(&self, format: PhoneNumberFormat) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format(self, format)
    }

    pub fn format_in_original_format(&self, region_calling_from: Region) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_in_original_format(self, region_calling_from)
    }

    pub fn format_national_with_carrier_code(&self, carrier_code: impl AsRef<str>) -> String {
        PHONE_NUMBER_UTIL.format_national_number_with_carrier_code(self, carrier_code)
    }

    pub fn format_for_mobile_dialing(
        &self,
        region_calling_from: Region,
        with_formatting: bool,
    ) -> Option<Cow<'_, str>> {
        PHONE_NUMBER_UTIL.format_number_for_mobile_dialing(
            self,
            region_calling_from,
            with_formatting,
        )
    }

    pub fn format_out_of_country_calling_number(
        &self,
        region_calling_from: Region,
    ) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_out_of_country_calling_number(self, region_calling_from)
    }

    pub fn format_out_of_country_keeping_alpha_chars(
        &self,
        region_calling_from: Region,
    ) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_out_of_country_keeping_alpha_chars(self, region_calling_from)
    }

    pub fn get_region_code(&self) -> Option<Region> {
        PHONE_NUMBER_UTIL.get_region_for_number(self)
    }

    pub fn get_type(&self) -> PhoneNumberType {
        PHONE_NUMBER_UTIL.get_number_type(self)
    }

    pub fn can_be_internationally_dialled(&self) -> bool {
        PHONE_NUMBER_UTIL.can_be_internationally_dialled(self)
    }

    pub fn is_geographical(&self) -> bool {
        PHONE_NUMBER_UTIL.is_number_geographical(self)
    }

    pub fn is_valid(&self) -> bool {
        PHONE_NUMBER_UTIL.is_valid_number(self)
    }

    pub fn is_valid_for_region(&self, region: Region) -> bool {
        PHONE_NUMBER_UTIL.is_valid_number_for_region(self, region)
    }

    pub fn is_possible_with_reason(&self) -> Result<NumberLengthType, ValidationError> {
        PHONE_NUMBER_UTIL.is_possible_number_with_reason(self)
    }

    pub fn truncate_too_long_number(&mut self) -> bool {
        PHONE_NUMBER_UTIL.truncate_too_long_number(self)
    }

    pub fn get_length_of_geographical_area_code(&self) -> usize {
        PHONE_NUMBER_UTIL.get_length_of_geographical_area_code(self)
    }

    pub fn get_length_of_national_destination_code(&self) -> usize {
        PHONE_NUMBER_UTIL.get_length_of_national_destination_code(self)
    }

    pub fn get_national_significant_number(&self) -> String {
        get_national_significant_number_owned(self)
    }

    pub fn parse(
        number_to_parse: impl AsRef<str>,
        default_region: Option<Region>,
    ) -> Result<PhoneNumber, ParseError> {
        PHONE_NUMBER_UTIL.parse(number_to_parse, default_region)
    }
}

impl FromStr for PhoneNumber {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s, None)
    }
}

impl Display for PhoneNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_as(PhoneNumberFormat::E164))
    }
}
