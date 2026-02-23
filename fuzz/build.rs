fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    cxx_build::bridge("fuzz_targets/diff-test.rs")
        .file("cpp/wrapper.cc")
        .include(&manifest_dir)
        .flag_if_supported("-std=c++17")
        .compile("phonenumber_cpp_bridge");

    println!("cargo:rustc-link-lib=phonenumber");
    println!("cargo:rustc-link-lib=protobuf");
    println!("cargo:rustc-link-lib=re2");

    println!("cargo:rerun-if-changed=fuzz_targets/diff-test.rs");
    println!("cargo:rerun-if-changed=cpp/wrapper.cc");
    println!("cargo:rerun-if-changed=cpp/wrapper.h");
}
