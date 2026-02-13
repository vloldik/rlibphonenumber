use std::str::FromStr;

use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};

use crate::{PhoneNumber, PhoneNumberFormat};

impl Serialize for PhoneNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&self.format_as(PhoneNumberFormat::E164))
    }
}

struct PhoneNumberVisitor {}
impl<'de> Visitor<'de> for PhoneNumberVisitor {
    type Value = PhoneNumber;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("valid E164 phone number")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        PhoneNumber::from_str(v).map_err(|err| E::custom(err.to_string()))
    }
}

impl<'de> Deserialize<'de> for PhoneNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(PhoneNumberVisitor {})
    }
}
