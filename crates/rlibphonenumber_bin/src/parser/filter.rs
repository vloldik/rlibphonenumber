use super::error::{MetadataError, Result};
use cel::{Context, Program, Value};
use rlibphonenumber::PhoneMetadata;

pub struct MetadataFilter {
    program: Option<Program>,
}

#[derive(Debug, Clone, Copy)]
struct MetadataContext<'a> {
    region: &'a str,
    country_code: i64,
    is_main_country: bool,
}

macro_rules! build_metadata_context {
    ($metadata:ident) => {{
        MetadataContext {
            region: &$metadata.id,
            country_code: $metadata.country_code.unwrap_or_default() as i64,
            is_main_country: $metadata.main_country_for_code.unwrap_or_default(),
        }
    }};
}

impl MetadataFilter {
    pub fn new(expression: &str) -> Result<Self> {
        let program =
            Program::compile(expression).map_err(|e| MetadataError::Cel(e.to_string()))?;
        Ok(Self {
            program: Some(program),
        })
    }

    pub fn empty_filter() -> Self {
        Self { program: None }
    }

    fn should_drop(
        &self,
        metadata_ctx: MetadataContext,
        parent: Option<&str>,
        field: Option<&str>,
    ) -> Result<bool> {
        let Some(prog) = &self.program else {
            return Ok(false);
        };

        let mut ctx = Context::default();
        macro_rules! add_variable {
            ($name:literal, $field: expr) => {
                ctx.add_variable($name, $field)
                    .map_err(|err| MetadataError::Cel(err.to_string()))?;
            };
        }

        add_variable!("field", field);
        add_variable!("parent", parent);
        add_variable!("region", metadata_ctx.region);
        add_variable!("country_code", metadata_ctx.country_code);
        add_variable!("is_main_country", metadata_ctx.is_main_country);

        match prog.execute(&ctx) {
            Ok(Value::Bool(drop_it)) => Ok(!drop_it),
            Ok(_) => Ok(false),
            Err(e) => Err(MetadataError::Cel(e.to_string())),
        }
    }

    pub fn filter_metadata(&self, mut metadata: PhoneMetadata) -> Result<Option<PhoneMetadata>> {
        let ctx = build_metadata_context!(metadata);
        if self.should_drop(ctx, None, None)? {
            return Ok(None);
        }

        macro_rules! filter_field {
            ($($field:ident),+) => {
                $(if self.should_drop(ctx, None, stringify!($field).into())? {
                    metadata.$field = None;
                })+
            };
        }

        filter_field!(
            preferred_international_prefix,
            national_prefix,
            preferred_extn_prefix,
            national_prefix_transform_rule,
            same_mobile_and_fixed_line_pattern,
            main_country_for_code,
            mobile_number_portable_region
        );

        #[allow(deprecated)]
        if self.should_drop(ctx, None, "number_format".into())? {
            metadata.number_format.clear();
        }
        #[allow(deprecated)]
        if self.should_drop(ctx, None, "intl_number_format".into())? {
            metadata.intl_number_format.clear();
        }

        macro_rules! filter_desc {
            ($($variant:ident),+) => {
                $(
                    #[allow(deprecated)]
                    if let Some(desc) = metadata.$variant.as_mut() {
                        if self.should_drop(ctx, stringify!($variant).into(), "national_number_pattern".into())? {
                            desc.national_number_pattern = None;
                        }
                        if self.should_drop(ctx, stringify!($variant).into(), "possible_length".into())? {
                            desc.possible_length.clear();
                        }
                        if self.should_drop(ctx, stringify!($variant).into(), "possible_length_local_only".into())? {
                            desc.possible_length_local_only.clear();
                        }
                        if self.should_drop(ctx, stringify!($variant).into(), "example_number".into())? {
                            desc.example_number = None;
                        }
                    }
                )+
            };
        }

        filter_desc!(
            general_desc,
            fixed_line,
            mobile,
            toll_free,
            premium_rate,
            shared_cost,
            personal_number,
            voip,
            pager,
            uan,
            emergency,
            voicemail,
            short_code,
            standard_rate,
            carrier_specific,
            sms_services,
            no_international_dialling
        );

        Ok(Some(metadata))
    }
}

pub fn get_metadata_filter(custom_filter: Option<&str>) -> Result<MetadataFilter> {
    if let Some(expr) = custom_filter {
        return MetadataFilter::new(expr);
    }
    Ok(MetadataFilter::empty_filter())
}
