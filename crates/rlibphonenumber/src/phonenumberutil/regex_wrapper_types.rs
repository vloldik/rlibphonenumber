use std::sync::OnceLock;

use crate::{
    NumberFormat, PhoneMetadata, PhoneNumberDesc, errors::InvalidRegexError, regexp::Regex,
};

#[derive(Debug, Clone)]
pub struct RegexTriplets {
    pub pattern_base: Option<String>,

    pub original: OnceLock<Result<Option<Regex>, crate::regexp::Error>>,
    pub anchor_start: OnceLock<Result<Option<Regex>, crate::regexp::Error>>,
    pub anchor_full: OnceLock<Result<Option<Regex>, crate::regexp::Error>>,
}

const MIN_LENGTH_FOR_WRAPPED_PATTERN: usize = 6; // ^(?:)$

impl RegexTriplets {
    pub fn new(pattern_base: Option<String>) -> Self {
        Self {
            pattern_base,
            original: OnceLock::new(),
            anchor_start: OnceLock::new(),
            anchor_full: OnceLock::new(),
        }
    }

    pub fn original_base(&self) -> &str {
        self.pattern_base
            .as_ref()
            .map(|base| {
                if base.len() >= MIN_LENGTH_FOR_WRAPPED_PATTERN {
                    &base[4..base.len() - 2]
                } else {
                    ""
                }
            })
            .unwrap_or_default()
    }

    pub fn new_vec(vec: Vec<String>) -> Vec<Self> {
        vec.into_iter().map(|s| Self::new(Some(s))).collect()
    }

    pub fn original(&self) -> Result<Option<&Regex>, InvalidRegexError> {
        self.original
            .get_or_init(|| {
                self.pattern_base
                    .as_ref()
                    .map(|base| {
                        if base.len() >= MIN_LENGTH_FOR_WRAPPED_PATTERN {
                            Regex::new(&base[1..base.len() - 1])
                        } else {
                            Regex::new("")
                        }
                    })
                    .transpose()
            })
            .as_ref()
            .map(|v| v.as_ref())
            .map_err(|e| e.clone().into())
    }

    pub fn anchor_start(&self) -> Result<Option<&Regex>, InvalidRegexError> {
        self.anchor_start
            .get_or_init(|| {
                self.pattern_base
                    .as_ref()
                    .map(|base| {
                        if base.len() >= MIN_LENGTH_FOR_WRAPPED_PATTERN {
                            Regex::new(&base[..base.len() - 1])
                        } else {
                            Regex::new("")
                        }
                    })
                    .transpose()
            })
            .as_ref()
            .map(|v| v.as_ref())
            .map_err(|e| e.clone().into())
    }

    pub fn anchor_full(&self) -> Result<Option<&Regex>, InvalidRegexError> {
        self.anchor_full
            .get_or_init(|| {
                self.pattern_base
                    .as_ref()
                    .map(|base| {
                        if base.len() >= MIN_LENGTH_FOR_WRAPPED_PATTERN {
                            Regex::new(base)
                        } else {
                            Regex::new("")
                        }
                    })
                    .transpose()
            })
            .as_ref()
            .map(|v| v.as_ref())
            .map_err(|e| e.clone().into())
    }
}

// Tf i was writing it for, but just leave it here...
macro_rules! wrapper {
    (struct $name:ident wraps $wraps:ty{
        $(Reg:
        $($field:ident <- $field_setter:ident),+

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
            $($field: RegexTriplets,)*
            $($($vec_field: Vec<RegexTriplets>,)*)?
            )?
            $($($(pub $extra: $extra_type,)*)*)?
            pub original: $wraps,
        }

        $(
        impl $name {
            $($(
            pub fn $vec_field(&self) -> &Vec<RegexTriplets> {
                    &self.$vec_field
            }
            )*)?
            $(
            pub fn $field(&self) -> &RegexTriplets {
                &self.$field
            }
            )*
            $(
            #[allow(dead_code)]
            pub fn $field_setter(&mut self, value: RegexTriplets) {
                self.$field = value;
            }
            )*
        }
        )?

        impl From<$wraps> for $name {
            #[allow(unused_mut, deprecated)]
            fn from(mut value: $wraps) -> Self {
                Self {
                    $(
                    $($field: RegexTriplets::new(::std::mem::take(&mut value.$field).into()),)*
                    $($($vec_field: RegexTriplets::new_vec(::std::mem::take(&mut value.$vec_field)),)*)?
                    )?
                    $($($(
                        $extra: {
                            let $extra_name = ::std::mem::take(&mut value.$extra);
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
    pattern <- set_pattern
Vec:
    leading_digits_pattern
});

wrapper!(struct PhoneNumberDescWrapper wraps PhoneNumberDesc {
Reg:
    national_number_pattern <- set_national_number_pattern
});

wrapper!(struct PhoneMetadataWrapper wraps PhoneMetadata {
Reg:
    leading_digits <- set_leading_digits,
    international_prefix <- set_international_prefix,
    national_prefix_for_parsing <- set_national_prefix_for_parsing
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
        desc.unwrap_or_default().into()
    }
});
