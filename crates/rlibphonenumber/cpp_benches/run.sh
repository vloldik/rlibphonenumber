dir="$(dirname $0)"
mkdir -p $dir/build && cd $dir/build
cmake ..
make -j$(nproc)

./bench_native
