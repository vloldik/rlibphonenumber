#pragma once
#include "rust/cxx.h"
#include <string>

#include "rlibphonenumber-fuzz/fuzz_targets/diff-test.rs.h"


CppResult test_cpp_impl(rust::Str number_str, rust::Str region_str);