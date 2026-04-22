use std::{env, fs, path::Path};

fn main() {
    let package_path = Path::new("resources/PhoneNumberMetadata.xml");
    if package_path.exists() {
        package_path
    } else {
        panic!(
            "Error: Could not find 'PhoneNumberMetadata.xml'. \
             Checked path: '{}'. \
             Make sure it is included in Cargo.toml",
            package_path.display()
        );
    };

    let file_content = fs::read_to_string(package_path).expect("Failed to read metadata file");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("PhoneNumberMetadata.xml");

    fs::write(dest_path, file_content).expect("Failed to write to OUT_DIR");
}
