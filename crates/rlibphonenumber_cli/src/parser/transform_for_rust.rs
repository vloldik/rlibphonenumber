use rlibphonenumber::{NumberFormat, PhoneMetadataCollection, PhoneNumberDesc};

fn wrap_regex_opt(pattern: &mut Option<String>) {
    if let Some(p) = pattern
        && !p.is_empty()
    {
        *p = format!("^(?:{})$", p);
    }
}

fn wrap_regex(pattern: &mut String) {
    if !pattern.is_empty() {
        *pattern = format!("^(?:{})$", pattern);
    }
}

#[allow(deprecated)]
fn transform_desc(desc: &mut Option<PhoneNumberDesc>) {
    if let Some(d) = desc {
        wrap_regex_opt(&mut d.national_number_pattern);
    }
}

#[allow(deprecated)]
fn transform_format(format: &mut NumberFormat) {
    wrap_regex(&mut format.pattern);

    for lp in &mut format.leading_digits_pattern {
        wrap_regex(lp);
    }
}

#[allow(deprecated)]
pub fn transform_for_rust(mut collection: PhoneMetadataCollection) -> PhoneMetadataCollection {
    for meta in &mut collection.metadata {
        wrap_regex_opt(&mut meta.leading_digits);
        wrap_regex_opt(&mut meta.international_prefix);
        wrap_regex_opt(&mut meta.national_prefix_for_parsing);

        transform_desc(&mut meta.general_desc);
        transform_desc(&mut meta.fixed_line);
        transform_desc(&mut meta.mobile);
        transform_desc(&mut meta.toll_free);
        transform_desc(&mut meta.premium_rate);
        transform_desc(&mut meta.shared_cost);
        transform_desc(&mut meta.personal_number);
        transform_desc(&mut meta.voip);
        transform_desc(&mut meta.pager);
        transform_desc(&mut meta.uan);
        transform_desc(&mut meta.emergency);
        transform_desc(&mut meta.voicemail);
        transform_desc(&mut meta.short_code);
        transform_desc(&mut meta.standard_rate);
        transform_desc(&mut meta.carrier_specific);
        transform_desc(&mut meta.sms_services);
        transform_desc(&mut meta.no_international_dialling);

        for fmt in &mut meta.number_format {
            transform_format(fmt);
        }

        for fmt in &mut meta.intl_number_format {
            transform_format(fmt);
        }
    }
    collection
}
