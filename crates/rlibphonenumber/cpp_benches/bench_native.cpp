#include <benchmark/benchmark.h>
#include <phonenumbers/phonenumberutil.h>
#include <string>
#include <vector>

using namespace i18n::phonenumbers;

struct TestEntity {
    std::string number;
    std::string region;
};

static const std::vector<TestEntity>& GetFormattingCases() {
    static const std::vector<TestEntity> cases = {
        {"+44 20 8765 4321", "GB"},
        {"020 8765 4321", "GB"},
        {"(650) 253-0000", "US"},
        {"02 12345678", "IT"},
        {"0011 54 9 11 8765 4321 ext. 1234", "AU"},
        {"011 15-1234-5678", "AR"},
        {"+1 (646) 222-3333 ext. 987", "US"},
        {"1-800-FLOWERS", "US"},
        {"1300 FLIGHT", "AU"},
        {" + 49 (0) 30 123456-78 ", "DE"},
        {"++41-44-668-18-00", "CH"},
        {"(03) 1234 5678", "JP"},
        {"+52 1 55 1234 5678", "MX"},
        {"054-123-4567", "IL"},
        {"+65 9123 4567", "SG"},
        {"416-555-0198", "CA"},
        {"242-322-1234", "BS"},
        {"12345", "DE"},
        {"112", "GB"},
        {"02-2123-4567", "KR"},
        {"0800 83 83 83", "NZ"},
        {"+7 916 123 4567", "RU"},
        {"+7 727 250 1234", "KZ"}
    };
    return cases;
}

static const std::vector<TestEntity>& GetParsingCases() {
    static const std::vector<TestEntity> cases = {
        {"0011 54 9 11 8765 4321 ext. 1234", "AU"},
        {"(650) 253-0000", "US"},
        {"+44 20 8765 4321", "GB"},
        {"020 8765 4321", "GB"},
        {"011 15-1234-5678", "AR"},
        {"02 12345678", "IT"},
        {"1-800-FLOWERS", "US"},
        {"12345", "DE"},
        {" + 49 (0) 30 123456-78 ", "DE"},
        {"++41-44-668-18-00", "CH"},
        {"+1 (646) 222-3333 ext. 987", "US"},
        {"112", "GB"}
    };
    return cases;
}

static const std::vector<PhoneNumber>& GetParsedNumbersForFormatting() {
    static std::vector<PhoneNumber> parsed_numbers;
    if (parsed_numbers.empty()) {
        PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();
        for (const auto& test : GetFormattingCases()) {
            PhoneNumber number;
            util->Parse(test.number, test.region, &number);
            parsed_numbers.push_back(number);
        }
    }
    return parsed_numbers;
}

static void BM_CreateUtil(benchmark::State& state) {
    for (auto _ : state) {
        PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();
        benchmark::DoNotOptimize(util);
    }
}
BENCHMARK(BM_CreateUtil);


static void BM_ParseComparison(benchmark::State& state) {
    PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();
    const auto& cases = GetParsingCases();
    
    PhoneNumber number;
    size_t index = 0;
    size_t total_cases = cases.size();

    for (auto _ : state) {
        util->Parse(cases[index].number, cases[index].region, &number);
        benchmark::DoNotOptimize(number);
        if (++index >= total_cases) index = 0;
    }
}
BENCHMARK(BM_ParseComparison);


static void BM_FormatComparison(benchmark::State& state, PhoneNumberUtil::PhoneNumberFormat format) {
    PhoneNumberUtil* util = PhoneNumberUtil::GetInstance();
    const auto& numbers_data = GetParsedNumbersForFormatting();
    
    std::string formatted;
    formatted.reserve(64); 
    
    size_t index = 0;
    size_t total_cases = numbers_data.size();

    for (auto _ : state) {
        util->Format(numbers_data[index], format, &formatted);
        benchmark::DoNotOptimize(formatted);
        
        if (++index >= total_cases) index = 0;
    }
}

BENCHMARK_CAPTURE(BM_FormatComparison, E164, PhoneNumberUtil::E164);
BENCHMARK_CAPTURE(BM_FormatComparison, International, PhoneNumberUtil::INTERNATIONAL);
BENCHMARK_CAPTURE(BM_FormatComparison, National, PhoneNumberUtil::NATIONAL);
BENCHMARK_CAPTURE(BM_FormatComparison, RFC3966, PhoneNumberUtil::RFC3966);


BENCHMARK_MAIN();