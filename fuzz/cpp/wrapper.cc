#include "wrapper.h"
#include <phonenumbers/phonenumberutil.h>

using namespace i18n::phonenumbers;

CppResult test_cpp_impl(rust::Str number_str, rust::Str region_str) {
    CppResult res;
    res.is_parsed = false;
    res.is_valid = false;
    res.is_possible = false;

    PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();
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
    }

    return res;
}