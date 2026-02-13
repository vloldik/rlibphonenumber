use std::{borrow::Cow, str::FromStr};

use crate::{
    NumberLengthType, PHONE_NUMBER_UTIL, ParseError, PhoneNumber, PhoneNumberFormat,
    PhoneNumberType, ValidationError,
};

pub trait PhoneNumberStaticExt {
    fn format(&self, format: PhoneNumberFormat) -> Cow<'_, str>;

    fn format_in_original_format(&self, region_calling_from: impl AsRef<str>) -> Cow<'_, str>;

    fn format_national_with_carrier_code(&self, carrier_code: impl AsRef<str>) -> String;

    fn format_for_mobile_dialing(
        &self,
        region_calling_from: impl AsRef<str>,
        with_formatting: bool,
    ) -> Cow<'_, str>;

    fn format_out_of_country_calling_number(
        &self,
        region_calling_from: impl AsRef<str>,
    ) -> Cow<'_, str>;

    fn format_out_of_country_keeping_alpha_chars(
        &self,
        region_calling_from: impl AsRef<str>,
    ) -> Cow<'_, str>;

    fn get_region_code<'a>(&self) -> Option<&'a str>;

    fn get_type(&self) -> PhoneNumberType;

    fn can_be_internationally_dialled(&self) -> bool;

    fn is_geographical(&self) -> bool;

    fn is_valid(&self) -> bool;

    fn is_valid_for_region(&self, region: impl AsRef<str>) -> bool;

    fn is_possible_with_reason(&self) -> Result<NumberLengthType, ValidationError>;

    fn truncate_too_long_number(&mut self) -> bool;

    fn get_length_of_geographical_area_code(&self) -> usize;

    fn get_length_of_national_destination_code(&self) -> usize;

    fn get_national_significant_number(&self) -> String;
}

impl PhoneNumberStaticExt for PhoneNumber {
    fn format(&self, format: PhoneNumberFormat) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format(self, format)
    }

    fn format_in_original_format(&self, region_calling_from: impl AsRef<str>) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_in_original_format(self, region_calling_from)
    }

    fn format_national_with_carrier_code(&self, carrier_code: impl AsRef<str>) -> String {
        PHONE_NUMBER_UTIL.format_national_number_with_carrier_code(self, carrier_code)
    }

    fn format_for_mobile_dialing(
        &self,
        region_calling_from: impl AsRef<str>,
        with_formatting: bool,
    ) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_number_for_mobile_dialing(
            self,
            region_calling_from,
            with_formatting,
        )
    }

    fn format_out_of_country_calling_number(
        &self,
        region_calling_from: impl AsRef<str>,
    ) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_out_of_country_calling_number(self, region_calling_from)
    }

    fn format_out_of_country_keeping_alpha_chars(
        &self,
        region_calling_from: impl AsRef<str>,
    ) -> Cow<'_, str> {
        PHONE_NUMBER_UTIL.format_out_of_country_keeping_alpha_chars(self, region_calling_from)
    }

    fn get_region_code<'a>(&self) -> Option<&'a str> {
        PHONE_NUMBER_UTIL.get_region_code_for_number(self)
    }

    fn get_type(&self) -> PhoneNumberType {
        PHONE_NUMBER_UTIL.get_number_type(self)
    }

    fn can_be_internationally_dialled(&self) -> bool {
        PHONE_NUMBER_UTIL.can_be_internationally_dialled(self)
    }

    fn is_geographical(&self) -> bool {
        PHONE_NUMBER_UTIL.is_number_geographical(self)
    }

    fn is_valid(&self) -> bool {
        PHONE_NUMBER_UTIL.is_valid_number(self)
    }

    fn is_valid_for_region(&self, region: impl AsRef<str>) -> bool {
        PHONE_NUMBER_UTIL.is_valid_number_for_region(self, region)
    }

    fn is_possible_with_reason(&self) -> Result<NumberLengthType, ValidationError> {
        PHONE_NUMBER_UTIL.is_possible_number_with_reason(self)
    }

    fn truncate_too_long_number(&mut self) -> bool {
        PHONE_NUMBER_UTIL.truncate_too_long_number(self)
    }

    fn get_length_of_geographical_area_code(&self) -> usize {
        PHONE_NUMBER_UTIL.get_length_of_geographical_area_code(self)
    }

    fn get_length_of_national_destination_code(&self) -> usize {
        PHONE_NUMBER_UTIL.get_length_of_national_destination_code(self)
    }

    fn get_national_significant_number(&self) -> String {
        PHONE_NUMBER_UTIL.get_national_significant_number(self)
    }
}

impl FromStr for PhoneNumber {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}
