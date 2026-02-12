use std::{env, fs};

fn main() {
    fs::copy(
        "../../resources/ShortNumberMetadata.xml",
        env::var("OUT_DIR").unwrap() + "/ShortNumberMetadata.xml",
    )
    .unwrap();
}
