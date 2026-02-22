use std::sync::OnceLock;

use paste::paste;
use regex::Regex;

use crate::{
    InvalidRegexError,
    phonemetadata::{NumberFormat, PhoneMetadata, PhoneNumberDesc},
};
type RegResult = Result<Regex, InvalidRegexError>;

// Tf i was writing it for, but just leave it here...
macro_rules! wrapper {
    (struct $name:ident wraps $wraps:ty{
        $(Reg:
        $($field:ident),+

        $(Vec:
        $($vec_field:ident),+
        )?)?

        $(Extra: $(
            $($extra:ident),*: $extra_type:ty | $extra_name:ident | { $extra_convert:expr }
        )*
        )?
    }) => {
        #[derive(Debug, Clone)]
        #[allow(dead_code)]
        pub struct $name {
            $(
            $($field: OnceLock<RegResult>,)*
            $($($vec_field: OnceLock<Vec<RegResult>>,)*)?
            )?
            $($($(pub $extra: $extra_type,)*)*)?
            pub original: $wraps,
        }

        $(
        impl $name {
            $($(
            pub fn $vec_field(&self) -> &Vec<RegResult> {
                self.$vec_field.get_or_init(|| {
                    self.original
                        .$vec_field
                        .iter()
                        .map(|pat| Regex::new(pat).map_err(| err | err.into()))
                        .collect()
                })
            }
            )*)?
            $(
            pub fn $field(&self) -> Result<&Regex, InvalidRegexError> {
                self.$field.get_or_init(|| {
                    Regex::new(
                        self.original
                       .$field()
                    ).map_err(|err| err.into())
                }).as_ref().map_err(| err | err.clone())
            }
            )*
            paste!{
                $(
                #[allow(dead_code)]
                pub fn [<set_ $field>](&mut self, value: OnceLock<RegResult>) {
                    self.$field = value;
                }
                )*
            }
        }
        )?

        impl From<$wraps> for $name {
            #[allow(unused_mut, deprecated)]
            fn from(mut value: $wraps) -> Self {
                Self {
                    $(
                    $($field: ::std::default::Default::default(),)*
                    $($($vec_field: ::std::default::Default::default(),)*)?
                    )?
                    $($($(
                        $extra: {
                            let $extra_name = value.$extra;
                            value.$extra = ::std::default::Default::default();
                            $extra_convert
                        },
                    )*)*)?
                    original: value,
                }
            }
        }

    };
}

wrapper!(struct NumberFormatWrapper wraps NumberFormat {
Reg:
    pattern
Vec:
    leading_digits_pattern
});

wrapper!(struct PhoneNumberDescWrapper wraps PhoneNumberDesc {
Reg:
    national_number_pattern
});

wrapper!(struct PhoneMetadataWrapper wraps PhoneMetadata {
Reg:
    leading_digits,
    international_prefix,
    national_prefix_for_parsing
Extra:
    number_format, intl_number_format: Vec<NumberFormatWrapper> | format | {
        format.into_iter().map(| v | v.into()).collect()
    }
    general_desc, fixed_line, mobile, toll_free, premium_rate,
    shared_cost, personal_number,
    voip, pager, uan, emergency,
    voicemail, short_code, standard_rate,
    carrier_specific, sms_services,
    no_international_dialling: PhoneNumberDescWrapper | desc | {
        desc.into_option().unwrap_or_default().into()
    }
});
