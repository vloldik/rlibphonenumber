#!/bin/bash

dir="$(dirname $0)"
cd $dir
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release

make -j$(nproc)

./build/bench_native
