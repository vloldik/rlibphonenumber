cmake_minimum_required(VERSION 3.14)
project(native_phonenumber_bench)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -O3 -DNDEBUG")

find_package(Protobuf REQUIRED)
add_executable(bench_native bench_native.cpp)

target_link_libraries(bench_native
    PRIVATE
    phonenumber
    benchmark
    protobuf::libprotobuf
    re2
    pthread
)