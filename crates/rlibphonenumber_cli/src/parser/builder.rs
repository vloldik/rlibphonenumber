#![allow(deprecated)]

use crate::sources::Source;

use super::{
    constants::*,
    error::{MetadataError, Result},
    filter::get_metadata_filter,
    utils::validate_re,
};
use rlibphonenumber::{NumberFormat, PhoneMetadata, PhoneMetadataCollection, PhoneNumberDesc};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::BTreeSet;

pub struct MetadataBuilder {
    custom_filter: Option<String>,
}

impl MetadataBuilder {
    pub fn new(custom_filter: Option<String>) -> Self {
        Self { custom_filter }
    }

    pub fn build_from_source(
        &self,
        source: Source,
        is_short_number: bool,
        is_alternate_formats: bool,
    ) -> Result<PhoneMetadataCollection> {
        let mut s = String::new();
        source.read()?.read_to_string(&mut s)?;
        self.build_collection(&s, is_short_number, is_alternate_formats)
    }

    pub fn build_collection(
        &self,
        xml_content: &str,
        is_short_number: bool,
        is_alternate_formats: bool,
    ) -> Result<PhoneMetadataCollection> {
        let doc = Document::parse_with_options(
            xml_content,
            ParsingOptions {
                allow_dtd: true,
                ..Default::default()
            },
        )?;
        let mut collection = PhoneMetadataCollection::default();
        let filter = get_metadata_filter(self.custom_filter.as_deref())?;

        let root = doc.root_element();

        for territory in root.descendants().filter(|n| n.has_tag_name("territory")) {
            let region_code = territory.attribute("id").unwrap_or("");
            let metadata = self.load_country_metadata(
                region_code,
                territory,
                is_short_number,
                is_alternate_formats,
            )?;

            if let Some(metadata) = filter.filter_metadata(metadata)? {
                collection.metadata.push(metadata);
            }
        }

        Ok(collection)
    }

    fn load_country_metadata(
        &self,
        region_code: &str,
        element: Node,
        is_short_number: bool,
        is_alternate_formats: bool,
    ) -> Result<PhoneMetadata> {
        let national_prefix = element.attribute(NATIONAL_PREFIX).unwrap_or("");
        let mut metadata =
            self.load_territory_tag_metadata(region_code, element, national_prefix)?;

        let np_formatting_rule = self.get_national_prefix_formatting_rule(element, national_prefix);
        let np_optional = element
            .attribute(NATIONAL_PREFIX_OPTIONAL_WHEN_FORMATTING)
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        self.load_available_formats(
            &mut metadata,
            element,
            national_prefix,
            &np_formatting_rule,
            np_optional,
        )?;

        if !is_alternate_formats {
            self.set_relevant_desc_patterns(&mut metadata, element, is_short_number)?;
        }

        Ok(metadata)
    }

    fn load_territory_tag_metadata(
        &self,
        region_code: &str,
        element: Node,
        national_prefix: &str,
    ) -> Result<PhoneMetadata> {
        let mut meta = PhoneMetadata {
            id: region_code.to_string(),
            ..Default::default()
        };

        if let Some(cc) = element.attribute(COUNTRY_CODE) {
            meta.country_code = Some(cc.parse().unwrap_or(0));
        }
        if let Some(ld) = element.attribute(LEADING_DIGITS) {
            meta.leading_digits = Some(validate_re(ld, false)?);
        }
        if let Some(ip) = element.attribute(INTERNATIONAL_PREFIX) {
            meta.international_prefix = Some(validate_re(ip, false)?);
        }
        if let Some(pip) = element.attribute(PREFERRED_INTERNATIONAL_PREFIX) {
            meta.preferred_international_prefix = Some(pip.to_string());
        }
        if let Some(npfp) = element.attribute(NATIONAL_PREFIX_FOR_PARSING) {
            meta.national_prefix_for_parsing = Some(validate_re(npfp, true)?);
            if let Some(nptr) = element.attribute(NATIONAL_PREFIX_TRANSFORM_RULE) {
                meta.national_prefix_transform_rule = Some(validate_re(nptr, false)?);
            }
        }
        if !national_prefix.is_empty() {
            meta.national_prefix = Some(national_prefix.to_string());
            if meta.national_prefix_for_parsing.is_none() {
                meta.national_prefix_for_parsing = Some(national_prefix.to_string());
            }
        }
        if let Some(pep) = element.attribute(PREFERRED_EXTN_PREFIX) {
            meta.preferred_extn_prefix = Some(pep.to_string());
        }
        if element.has_attribute(MAIN_COUNTRY_FOR_CODE) {
            meta.main_country_for_code = Some(true);
        }
        if element.has_attribute(MOBILE_NUMBER_PORTABLE_REGION) {
            meta.mobile_number_portable_region = Some(true);
        }

        Ok(meta)
    }

    fn load_available_formats(
        &self,
        metadata: &mut PhoneMetadata,
        element: Node,
        national_prefix: &str,
        parent_np_rule: &str,
        parent_np_optional: bool,
    ) -> Result<()> {
        let mut carrier_rule = String::new();
        if element.has_attribute(CARRIER_CODE_FORMATTING_RULE) {
            let rule = self.get_domestic_carrier_rule(element, national_prefix);
            carrier_rule = validate_re(&rule, false)?;
        }

        let format_nodes: Vec<_> = element
            .descendants()
            .filter(|n| n.has_tag_name(NUMBER_FORMAT))
            .collect();
        let mut has_explicit_intl = false;
        let is_nodes_empty = format_nodes.is_empty();

        for node in format_nodes {
            let mut format = NumberFormat::default();

            if node.has_attribute(NATIONAL_PREFIX_FORMATTING_RULE) {
                format.national_prefix_formatting_rule =
                    Some(self.get_national_prefix_formatting_rule(node, national_prefix));
            } else if !parent_np_rule.is_empty() {
                format.national_prefix_formatting_rule = Some(parent_np_rule.to_string());
            }

            if let Some(opt) = node.attribute(NATIONAL_PREFIX_OPTIONAL_WHEN_FORMATTING) {
                format.national_prefix_optional_when_formatting =
                    Some(opt.parse().unwrap_or(false));
            } else {
                format.national_prefix_optional_when_formatting = Some(parent_np_optional);
            }

            if node.has_attribute(CARRIER_CODE_FORMATTING_RULE) {
                let rule = self.get_domestic_carrier_rule(node, national_prefix);
                format.domestic_carrier_code_formatting_rule = Some(validate_re(&rule, false)?);
            } else if !carrier_rule.is_empty() {
                format.domestic_carrier_code_formatting_rule = Some(carrier_rule.clone());
            }

            self.load_national_format(metadata, node, &mut format)?;
            metadata.number_format.push(format.clone());

            if self.load_international_format(metadata, node, &format)? {
                has_explicit_intl = true;
            }
        }

        if !is_nodes_empty && !has_explicit_intl {
            metadata.intl_number_format.clear();
        }
        Ok(())
    }

    fn load_national_format(
        &self,
        metadata: &PhoneMetadata,
        node: Node,
        format: &mut NumberFormat,
    ) -> Result<()> {
        self.set_leading_digits_patterns(node, format)?;
        if let Some(pat) = node.attribute(PATTERN) {
            format.pattern = validate_re(pat, false)?;
        }

        let format_nodes: Vec<_> = node.children().filter(|n| n.has_tag_name(FORMAT)).collect();
        if format_nodes.len() != 1 {
            let id = if metadata.id.is_empty() {
                metadata.country_code.unwrap_or(0).to_string()
            } else {
                metadata.id.clone()
            };
            return Err(MetadataError::Build(format!(
                "Invalid number of format patterns for country: {}",
                id
            )));
        }

        format.format = format_nodes[0].text().unwrap_or("").to_string();
        Ok(())
    }

    fn load_international_format(
        &self,
        metadata: &mut PhoneMetadata,
        node: Node,
        national_format: &NumberFormat,
    ) -> Result<bool> {
        let mut intl_format = national_format.clone();
        let intl_nodes: Vec<_> = node
            .children()
            .filter(|n| n.has_tag_name(INTL_FORMAT))
            .collect();

        if intl_nodes.len() > 1 {
            let id = if metadata.id.is_empty() {
                metadata.country_code.unwrap_or(0).to_string()
            } else {
                metadata.id.clone()
            };
            return Err(MetadataError::Build(format!(
                "Invalid number of intlFormat patterns for country: {}",
                id
            )));
        }

        let mut has_explicit_intl = false;

        if !intl_nodes.is_empty() {
            intl_format.pattern = node.attribute(PATTERN).unwrap_or("").to_string();
            intl_format.leading_digits_pattern.clear();
            self.set_leading_digits_patterns(node, &mut intl_format)?;

            let val = intl_nodes[0].text().unwrap_or("");
            if val != "NA" {
                intl_format.format = val.to_string();
            } else {
                intl_format.format.clear();
            }
            has_explicit_intl = true;
        }

        if !intl_format.format.is_empty() {
            metadata.intl_number_format.push(intl_format);
        }

        Ok(has_explicit_intl)
    }

    fn set_relevant_desc_patterns(
        &self,
        metadata: &mut PhoneMetadata,
        element: Node,
        is_short: bool,
    ) -> Result<()> {
        let mut gen_desc = self.process_desc(None, element, GENERAL_DESC)?;
        self.set_possible_lengths_general_desc(&mut gen_desc, &metadata.id, element, is_short)?;
        metadata.general_desc = Some(gen_desc.clone());
        let parent = Some(&gen_desc);

        if !is_short {
            metadata.fixed_line = Some(self.process_desc(parent, element, FIXED_LINE)?);
            metadata.mobile = Some(self.process_desc(parent, element, MOBILE)?);
            metadata.shared_cost = Some(self.process_desc(parent, element, SHARED_COST)?);
            metadata.voip = Some(self.process_desc(parent, element, VOIP)?);
            metadata.personal_number = Some(self.process_desc(parent, element, PERSONAL_NUMBER)?);
            metadata.pager = Some(self.process_desc(parent, element, PAGER)?);
            metadata.uan = Some(self.process_desc(parent, element, UAN)?);
            metadata.voicemail = Some(self.process_desc(parent, element, VOICEMAIL)?);
            metadata.no_international_dialling =
                Some(self.process_desc(parent, element, NO_INTERNATIONAL_DIALLING)?);

            let mobile_pat = metadata
                .mobile
                .as_ref()
                .and_then(|m| m.national_number_pattern.clone())
                .unwrap_or_default();
            let fixed_pat = metadata
                .fixed_line
                .as_ref()
                .and_then(|f| f.national_number_pattern.clone())
                .unwrap_or_default();
            metadata.same_mobile_and_fixed_line_pattern =
                Some(!mobile_pat.is_empty() && mobile_pat == fixed_pat);

            metadata.toll_free = Some(self.process_desc(parent, element, TOLL_FREE)?);
            metadata.premium_rate = Some(self.process_desc(parent, element, PREMIUM_RATE)?);
        } else {
            metadata.standard_rate = Some(self.process_desc(parent, element, STANDARD_RATE)?);
            metadata.short_code = Some(self.process_desc(parent, element, SHORT_CODE)?);
            metadata.carrier_specific =
                Some(self.process_desc(parent, element, CARRIER_SPECIFIC)?);
            metadata.emergency = Some(self.process_desc(parent, element, EMERGENCY)?);
            metadata.toll_free = Some(self.process_desc(parent, element, TOLL_FREE)?);
            metadata.premium_rate = Some(self.process_desc(parent, element, PREMIUM_RATE)?);
            metadata.sms_services = Some(self.process_desc(parent, element, SMS_SERVICES)?);
        }
        Ok(())
    }

    fn process_desc(
        &self,
        parent: Option<&PhoneNumberDesc>,
        element: Node,
        tag: &str,
    ) -> Result<PhoneNumberDesc> {
        let mut desc = PhoneNumberDesc::default();
        let nodes: Vec<_> = element.children().filter(|n| n.has_tag_name(tag)).collect();

        if nodes.is_empty() {
            desc.possible_length.push(-1);
            return Ok(desc);
        }
        if nodes.len() > 1 {
            return Err(MetadataError::Build(format!(
                "Multiple elements with type {} found.",
                tag
            )));
        }

        let node = nodes[0];
        if let Some(p) = parent {
            let mut lengths = BTreeSet::new();
            let mut local = BTreeSet::new();
            for n in node
                .descendants()
                .filter(|x| x.has_tag_name(POSSIBLE_LENGTHS))
            {
                self.populate_length_node(n, &mut lengths, &mut local)?;
            }
            self.set_possible_lengths(lengths, local, Some(p), &mut desc)?;
        }

        if let Some(pat) = node
            .children()
            .find(|n| n.has_tag_name(NATIONAL_NUMBER_PATTERN))
            && let Some(text) = pat.text()
        {
            desc.national_number_pattern = Some(validate_re(text, true)?);
        }

        if let Some(ex) = node.children().find(|n| n.has_tag_name(EXAMPLE_NUMBER)) {
            desc.example_number = ex.text().map(|s| s.to_string());
        }

        Ok(desc)
    }

    fn set_possible_lengths_general_desc(
        &self,
        general_desc: &mut PhoneNumberDesc,
        meta_id: &str,
        data: Node,
        is_short: bool,
    ) -> Result<()> {
        let mut lengths = BTreeSet::new();
        let mut local = BTreeSet::new();

        if let Some(gen_node) = data.children().find(|n| n.has_tag_name(GENERAL_DESC)) {
            for p in gen_node
                .descendants()
                .filter(|n| n.has_tag_name(POSSIBLE_LENGTHS))
            {
                self.populate_length_node(p, &mut lengths, &mut local)?;
            }
            if !lengths.is_empty() || !local.is_empty() {
                return Err(MetadataError::Build(format!(
                    "Found possible lengths specified at general desc. Country: {}",
                    meta_id
                )));
            }
        }

        if !is_short {
            for p in data
                .descendants()
                .filter(|n| n.has_tag_name(POSSIBLE_LENGTHS))
            {
                let p_tag = p
                    .parent_element()
                    .map(|x| x.tag_name().name())
                    .unwrap_or("");
                if !PHONE_NUMBER_DESCS_WITHOUT_MATCHING_TYPES.contains(&p_tag) {
                    self.populate_length_node(p, &mut lengths, &mut local)?;
                }
            }
        } else {
            if let Some(sc) = data.children().find(|n| n.has_tag_name(SHORT_CODE)) {
                for p in sc
                    .descendants()
                    .filter(|n| n.has_tag_name(POSSIBLE_LENGTHS))
                {
                    self.populate_length_node(p, &mut lengths, &mut local)?;
                }
            }
            if !local.is_empty() {
                return Err(MetadataError::Build(
                    "Found local-only lengths in short-number metadata".into(),
                ));
            }
        }

        self.set_possible_lengths(lengths, local, None, general_desc)?;
        Ok(())
    }

    fn populate_length_node(
        &self,
        element: Node,
        lengths: &mut BTreeSet<i32>,
        local: &mut BTreeSet<i32>,
    ) -> Result<()> {
        if let Some(nat) = element.attribute(NATIONAL) {
            let l = self.parse_length_string(nat)?;
            if let Some(loc) = element.attribute(LOCAL_ONLY) {
                let loc_set = self.parse_length_string(loc)?;
                if !l.is_disjoint(&loc_set) {
                    return Err(MetadataError::Build(format!(
                        "Overlap in lengths: {:?}",
                        l.intersection(&loc_set).collect::<Vec<_>>()
                    )));
                }
                local.extend(loc_set);
            }
            lengths.extend(l);
        }
        Ok(())
    }

    fn set_possible_lengths(
        &self,
        lengths: BTreeSet<i32>,
        local: BTreeSet<i32>,
        parent: Option<&PhoneNumberDesc>,
        desc: &mut PhoneNumberDesc,
    ) -> Result<()> {
        desc.possible_length.clear();
        desc.possible_length_local_only.clear();

        let are_equal = parent.is_some_and(|p| {
            p.possible_length.len() == lengths.len()
                && lengths.iter().zip(&p.possible_length).all(|(a, b)| a == b)
        });

        if parent.is_none() || !are_equal {
            for &len in &lengths {
                if let Some(p) = parent {
                    if p.possible_length.contains(&len) {
                        desc.possible_length.push(len);
                    } else {
                        return Err(MetadataError::Build(format!(
                            "Out-of-range possible length found ({})",
                            len
                        )));
                    }
                } else {
                    desc.possible_length.push(len);
                }
            }
        }

        for &len in &local {
            if !lengths.contains(&len) {
                if let Some(p) = parent {
                    if p.possible_length_local_only.contains(&len)
                        || p.possible_length.contains(&len)
                    {
                        desc.possible_length_local_only.push(len);
                    } else {
                        return Err(MetadataError::Build(format!(
                            "Out-of-range local-only possible length found ({})",
                            len
                        )));
                    }
                } else {
                    desc.possible_length_local_only.push(len);
                }
            }
        }
        Ok(())
    }

    fn parse_length_string(&self, s: &str) -> Result<BTreeSet<i32>> {
        if s.is_empty() {
            return Err(MetadataError::Build(
                "Empty possibleLength string found.".into(),
            ));
        }
        let mut set = BTreeSet::new();

        for part in s.split(',') {
            if part.is_empty() {
                return Err(MetadataError::Build(
                    "Leading, trailing or adjacent commas found.".into(),
                ));
            }
            if part.starts_with('[') && part.ends_with(']') {
                let inner = &part[1..part.len() - 1];
                let bounds: Vec<&str> = inner.split('-').collect();
                if bounds.len() != 2 {
                    return Err(MetadataError::Build(
                        "Ranges must have exactly one - character".into(),
                    ));
                }
                let min: i32 = bounds[0].parse()?;
                let max: i32 = bounds[1].parse()?;
                if max - min < 2 {
                    return Err(MetadataError::Build(
                        "First number should be 2+ lower than second".into(),
                    ));
                }
                for i in min..=max {
                    if !set.insert(i) {
                        return Err(MetadataError::Build(format!(
                            "Duplicate length element found: {}",
                            i
                        )));
                    }
                }
            } else {
                let val: i32 = part.parse()?;
                if !set.insert(val) {
                    return Err(MetadataError::Build(format!(
                        "Duplicate length element found: {}",
                        val
                    )));
                }
            }
        }
        Ok(set)
    }

    fn set_leading_digits_patterns(&self, node: Node, format: &mut NumberFormat) -> Result<()> {
        for n in node.children().filter(|n| n.has_tag_name(LEADING_DIGITS)) {
            if let Some(text) = n.text() {
                format.leading_digits_pattern.push(validate_re(text, true)?);
            }
        }
        Ok(())
    }

    fn get_national_prefix_formatting_rule(&self, element: Node, np: &str) -> String {
        let rule = element
            .attribute(NATIONAL_PREFIX_FORMATTING_RULE)
            .unwrap_or("");
        rule.replacen("$NP", np, 1).replacen("$FG", "$1", 1)
    }

    fn get_domestic_carrier_rule(&self, element: Node, np: &str) -> String {
        let rule = element
            .attribute(CARRIER_CODE_FORMATTING_RULE)
            .unwrap_or("");
        rule.replacen("$FG", "$1", 1).replacen("$NP", np, 1)
    }
}
