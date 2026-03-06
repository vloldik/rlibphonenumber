#pragma once
#include "rust/cxx.h"
#include <string>

#include "rlibphonenumber-fuzz/src/lib.rs.h"


CppResult test_cpp_impl(rust::Str number_str, rust::Str region_str);
bool bench_cpp_pure(rust::Str number_str, rust::Str region_str);