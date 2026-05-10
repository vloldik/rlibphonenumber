use std::{env, fs, path::Path};

fn main() {
    copy_resource_file("PhoneNumberMetadata.xml");
    copy_resource_file("countries.csv");
}

fn copy_resource_file(file_name: &str) {
    let path = &format!("resources/{}", file_name);
    let package_path = Path::new(path);
    if package_path.exists() {
        package_path
    } else {
        panic!(
            "Error: Could not find '{}'. \
             Checked path: '{}'. \
             Make sure it is included in Cargo.toml",
            file_name,
            package_path.display()
        );
    };

    let file_content = fs::read_to_string(package_path).expect("Failed to read metadata file");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(file_name);

    fs::write(dest_path, file_content).expect("Failed to write to OUT_DIR");
}
