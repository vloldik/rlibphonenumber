#pragma once
#include "rust/cxx.h"
#include <string>

#include "rlibphonenumber-fuzz/src/lib.rs.h"


CppResult test_cpp_impl(rust::Str number_str, rust::Str region_str);
rust::Vec<MatchResult> test_cpp_matcher(rust::Str text_str, rust::Str region_str);