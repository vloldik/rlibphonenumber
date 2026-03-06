mkdir -p build && cd build
cmake ..
make -j$(nproc)

./bench_native
