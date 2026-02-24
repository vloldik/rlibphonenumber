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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::str::FromStr;

    fn get_valid_number_str() -> &'static str {
        "+12025550100"
    }

    #[test]
    fn test_serialize_phone_number() {
        let raw_num = get_valid_number_str();
        let phone = PhoneNumber::from_str(raw_num).expect("Failed to create phone number from str");

        let serialized = serde_json::to_string(&phone).expect("Failed to serialize");
        let expected_json = format!("\"{}\"", raw_num);
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn test_deserialize_phone_number() {
        let raw_num = get_valid_number_str();
        let json_input = format!("\"{}\"", raw_num);

        let phone: PhoneNumber = serde_json::from_str(&json_input).expect("Failed to deserialize");

        assert_eq!(phone.format_as(PhoneNumberFormat::E164), raw_num);
    }

    #[test]
    fn test_deserialize_invalid_format() {
        let json_input = "\"not-a-phone-number\"";

        let result: Result<PhoneNumber, _> = serde_json::from_str(json_input);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_type() {
        let json_input = "123456789";

        let result: Result<PhoneNumber, _> = serde_json::from_str(json_input);

        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip() {
        let raw_num = get_valid_number_str();
        let original_phone = PhoneNumber::from_str(raw_num).unwrap();

        let serialized = serde_json::to_string(&original_phone).unwrap();
        let deserialized: PhoneNumber = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            original_phone.format_as(PhoneNumberFormat::E164),
            deserialized.format_as(PhoneNumberFormat::E164)
        );
    }

    #[test]
    fn test_struct_integration() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct User {
            name: String,
            phone: PhoneNumber,
        }

        let raw_num = get_valid_number_str();
        let user = User {
            name: "Alice".to_string(),
            phone: PhoneNumber::from_str(raw_num).unwrap(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"phone\":\"+"));

        let deserialized_user: User = serde_json::from_str(&json).unwrap();
        assert_eq!(user.name, deserialized_user.name);

        assert_eq!(
            user.phone.format_as(PhoneNumberFormat::E164),
            deserialized_user.phone.format_as(PhoneNumberFormat::E164)
        );
    }
}
