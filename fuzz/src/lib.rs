#[cxx::bridge]
pub mod ffi {
    #[derive(Debug)]
    struct CppResult {
        is_parsed: bool,
        is_valid: bool,
        is_possible: bool,
        region_code: String,
        nsn: String,
        format_e164: String,
        format_intl: String,
        format_natl: String,
        format_rfc3966: String,
        format_mobile: String,
        out_of_country_keeping_alpha: String,

        error: String,
    }

    #[derive(Debug)]
    pub struct MatchResult {
        pub start: i32,
        pub end: i32,
        pub raw_string: String,
        pub e164: String,
    }

    unsafe extern "C++" {
        include!("cpp/wrapper.h");
        fn test_cpp_impl(number_str: &str, region_str: &str) -> CppResult;
        fn test_cpp_matcher(number_str: &str, region_str: &str) -> Vec<MatchResult>;

    }
}
