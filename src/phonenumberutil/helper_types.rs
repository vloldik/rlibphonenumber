use std::borrow::Cow;

use crate::proto_gen::phonenumber::phone_number::CountryCodeSource;

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

#[derive(Debug)]
pub struct PhoneNumberAndCarrierCode<'a> {
    pub carrier_code: Option<&'a str>,
    pub phone_number: Cow<'a, str>
}

impl<'a> PhoneNumberAndCarrierCode<'a> {
    pub fn new<B: Into<Cow<'a, str>>>(carrier_code: Option<&'a str>, phone_number: B) -> Self {
        Self { carrier_code, phone_number: phone_number.into() }
    }
    
    pub fn new_phone<B: Into<Cow<'a, str>>>(phone_number: B) -> Self {
        Self { carrier_code: None, phone_number: phone_number.into() }
    }
}