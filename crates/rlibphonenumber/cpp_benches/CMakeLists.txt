cmake_minimum_required(VERSION 3.14)
project(native_phonenumber_bench)

set(CMAKE_CXX_STANDARD 17)

find_package(Protobuf REQUIRED)

include(FetchContent)
FetchContent_Declare(
  google_benchmark
  GIT_REPOSITORY https://github.com/google/benchmark.git
  GIT_TAG        v1.8.3
)
set(BENCHMARK_ENABLE_TESTING OFF CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(google_benchmark)

add_executable(bench_native bench_native.cpp)

target_link_libraries(bench_native
    PRIVATE
    phonenumber
    benchmark::benchmark
    protobuf::libprotobuf
    re2
    pthread
)