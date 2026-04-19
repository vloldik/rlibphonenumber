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

    pub fn for_lite_build() -> Result<Self> {
        Self::new("field == 'example_number'")
    }

    pub fn for_special_build() -> Result<Self> {
        Self::new("parent != '' && parent != 'mobile'")
    }

    fn should_drop(
        &self,
        metadata_ctx: MetadataContext,
        parent: &str,
        field: &str,
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
            Ok(Value::Bool(drop_it)) => Ok(drop_it),
            Ok(_) => Ok(false),
            Err(e) => Err(MetadataError::Cel(e.to_string())),
        }
    }

    pub fn filter_metadata(&self, metadata: &mut PhoneMetadata) -> Result<()> {
        let ctx = build_metadata_context!(metadata);

        if self.should_drop(ctx, "", "preferred_international_prefix")? {
            metadata.preferred_international_prefix = None;
        }
        if self.should_drop(ctx, "", "national_prefix")? {
            metadata.national_prefix = None;
        }
        if self.should_drop(ctx, "", "preferred_extn_prefix")? {
            metadata.preferred_extn_prefix = None;
        }
        if self.should_drop(ctx, "", "national_prefix_transform_rule")? {
            metadata.national_prefix_transform_rule = None;
        }
        if self.should_drop(ctx, "", "same_mobile_and_fixed_line_pattern")? {
            metadata.same_mobile_and_fixed_line_pattern = None;
        }
        if self.should_drop(ctx, "", "main_country_for_code")? {
            metadata.main_country_for_code = None;
        }
        if self.should_drop(ctx, "", "mobile_number_portable_region")? {
            metadata.mobile_number_portable_region = None;
        }

        macro_rules! filter_desc {
            ($($variant:ident),+) => {
                $(
                    #[allow(deprecated)]
                    if let Some(desc) = metadata.$variant.as_mut() {
                        if self.should_drop(ctx, stringify!($variant), "national_number_pattern")? {
                            desc.national_number_pattern = None;
                        }
                        if self.should_drop(ctx, stringify!($variant), "possible_length")? {
                            desc.possible_length.clear();
                        }
                        if self.should_drop(ctx, stringify!($variant), "possible_length_local_only")? {
                            desc.possible_length_local_only.clear();
                        }
                        if self.should_drop(ctx, stringify!($variant), "example_number")? {
                            desc.example_number = None;
                        }
                    }
                )+
            };
        }

        filter_desc!(
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

        Ok(())
    }
}

pub fn get_metadata_filter(lite_build: bool, special_build: bool) -> Result<MetadataFilter> {
    if special_build {
        if lite_build {
            return Err(MetadataError::Validation(
                "liteBuild and specialBuild may not both be set".into(),
            ));
        }
        MetadataFilter::for_special_build()
    } else if lite_build {
        MetadataFilter::for_lite_build()
    } else {
        Ok(MetadataFilter::empty_filter())
    }
}
