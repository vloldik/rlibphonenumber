use std::{env, fs, path::Path};

fn main() {
    let workspace_path = Path::new("../../resources/ShortNumberMetadata.xml");
    let package_path = Path::new("resources/ShortNumberMetadata.xml");
    let source_path = if workspace_path.exists() {
        workspace_path
    } else if package_path.exists() {
        package_path
    } else {
        panic!(
            "Error: Could not find 'ShortNumberMetadata.xml'. \
             Checked paths: '{}' and '{}'. \
             Make sure it is included in Cargo.toml",
            workspace_path.display(),
            package_path.display()
        );
    };

    println!("cargo:rerun-if-changed={}", source_path.display());
    let file_content = fs::read_to_string(source_path).expect("Failed to read metadata file");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("ShortNumberMetadata.xml");

    fs::write(dest_path, file_content).expect("Failed to write to OUT_DIR");
}
