use std::sync::Arc;

use super::regex_and_mappings::PhoneNumberRegExpsAndMappings;
use crate::{interfaces::MatcherApi, proto_gen::phonemetadata::PhoneMetadata};

use dashmap::{DashMap, DashSet};

pub struct PhoneNumberUtil {
    /// An API for validation checking.
    matcher_api: Box<dyn MatcherApi>,

    /// Helper class holding useful regular expressions and character mappings.
    reg_exps: Arc<PhoneNumberRegExpsAndMappings>,

    /// A mapping from a country calling code to a RegionCode object which denotes
    /// the region represented by that country calling code. Note regions under
    /// NANPA share the country calling code 1 and Russia and Kazakhstan share the
    /// country calling code 7. Under this map, 1 is mapped to region code "US" and
    /// 7 is mapped to region code "RU". This is implemented as a sorted vector to
    /// achieve better performance.
    country_calling_code_to_region_code_map: Vec<(i32, String)>,

    /// The set of regions that share country calling code 1.
    nanpa_regions: DashSet<String>,

    /// A mapping from a region code to a PhoneMetadata for that region.
    region_to_metadata_map: DashMap<String, PhoneMetadata>,

    // A mapping from a country calling code for a non-geographical entity to the
    // PhoneMetadata for that country calling code. Examples of the country
    // calling codes include 800 (International Toll Free Service) and 808
    // (International Shared Cost Service).
    country_code_to_non_geographical_metadata_map: DashMap<u32, PhoneMetadata>,
}
