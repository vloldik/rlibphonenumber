#include "wrapper.h"
#include <phonenumbers/phonenumberutil.h>
#include <phonenumbers/phonenumbermatch.h>
#include <phonenumbers/phonenumbermatcher.h>

using namespace i18n::phonenumbers;

std::string ErrorTypeToString(PhoneNumberUtil::ErrorType error) {
    switch (error) {
        case PhoneNumberUtil::NO_PARSING_ERROR:
            return "None";
        case PhoneNumberUtil::INVALID_COUNTRY_CODE_ERROR:
            return "INVALID_COUNTRY_CODE_ERROR";
        case PhoneNumberUtil::NOT_A_NUMBER:
            return "NOT_A_NUMBER";
        case PhoneNumberUtil::TOO_SHORT_AFTER_IDD:
            return "TOO_SHORT_AFTER_IDD";
        case PhoneNumberUtil::TOO_SHORT_NSN:
            return "TOO_SHORT_NSN";
        case PhoneNumberUtil::TOO_LONG:
            return "TOO_LONG";
        default:
            return "Unknown error";
    }
}

CppResult test_cpp_impl(rust::Str number_str, rust::Str region_str) {
    CppResult res;
    res.is_parsed = false;
    res.is_valid = false;
    res.is_possible = false;

    static PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();
    
    PhoneNumber number;

    std::string num_std(number_str.data(), number_str.size());
    std::string reg_std(region_str.data(), region_str.size());

    auto status = util->Parse(num_std, reg_std, &number);

    if (status == PhoneNumberUtil::NO_PARSING_ERROR) {
        res.is_parsed = true;
        res.is_valid = util->IsValidNumber(number);
        res.is_possible = util->IsPossibleNumber(number);

        std::string region_out;
        util->GetRegionCodeForNumber(number, &region_out);
        res.region_code = region_out;

        std::string nsn_out;
        util->GetNationalSignificantNumber(number, &nsn_out);
        res.nsn = nsn_out;

        std::string fmt;
        util->Format(number, PhoneNumberUtil::E164, &fmt);
        res.format_e164 = fmt;

        util->Format(number, PhoneNumberUtil::INTERNATIONAL, &fmt);
        res.format_intl = fmt;

        util->Format(number, PhoneNumberUtil::NATIONAL, &fmt);
        res.format_natl = fmt;

        util->Format(number, PhoneNumberUtil::RFC3966, &fmt);
        res.format_rfc3966 = fmt;

        std::string mobile_fmt;
        util->FormatNumberForMobileDialing(number, reg_std, true, &mobile_fmt);
        res.format_mobile = mobile_fmt;

        std::string ouc_alpha;
        util->FormatOutOfCountryKeepingAlphaChars(number, reg_std, &ouc_alpha);
        res.out_of_country_keeping_alpha = ouc_alpha;
    } else { 
        res.error = ErrorTypeToString(status); 
    }

    return res;
}


rust::Vec<MatchResult> test_cpp_matcher(rust::Str text_str, rust::Str region_str) {
    static PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();

    std::string text(text_str.data(),   text_str.size());
    std::string region(region_str.data(), region_str.size());

    PhoneNumberMatcher matcher(*util, text, region,
                               PhoneNumberMatcher::VALID,
                               /*max_tries=*/std::numeric_limits<int>::max());

    rust::Vec<MatchResult> results;
    while (matcher.HasNext()) {
        PhoneNumberMatch match;
        matcher.Next(&match);

        MatchResult r;
        r.start      = static_cast<int32_t>(match.start());
        r.end        = static_cast<int32_t>(match.end());
        r.raw_string = match.raw_string();

        std::string e164;
        util->Format(match.number(), PhoneNumberUtil::E164, &e164);
        r.e164 = e164;

        results.push_back(r);
    }
    return results;
}