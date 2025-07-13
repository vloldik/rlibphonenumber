use std::borrow::Cow;

use crate::phonenumber::phone_number::CountryCodeSource;

#[derive(Debug)]
pub struct PhoneNumberWithCountryCodeSource<'a> {
    pub phone_number: Cow<'a, str>,
    pub country_code_source: CountryCodeSource
}

impl<'a> PhoneNumberWithCountryCodeSource<'a> {
    pub fn new(phone_number: Cow<'a, str>, country_code_source: CountryCodeSource) -> Self {
        Self { phone_number, country_code_source }
    }
}
