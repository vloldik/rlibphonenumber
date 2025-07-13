#!/bin/bash

filedir="./$(dirname "$0")"
javadir="$filedir/../java"
project_home="$filedir/../.."
generated_dir="$project_home/src/generated/metadata"
echo $generated_dir

resources_dir="$project_home/resources"
rust_build_jar="$javadir/rust-build/target/rust-build-1.0-SNAPSHOT-jar-with-dependencies.jar"

# mvn -f "$javadir/pom.xml" install
mkdir -p "$generated_dir"

function generate {
    java -jar "$rust_build_jar" \
        BuildMetadataRustFromXml \
        "$resources_dir/$1" \
        "$generated_dir/$2.rs" \
        "$3" \
        "--const-name=$4"
}

# generate general metadata
generate "PhoneNumberMetadata.xml" "metadata" "metadata" "METADATA"

# generate short metadata
generate "PhoneNumberMetadataForTesting.xml" "test_metadata" "metadata" "TEST_METADATA"

echo "\
mod metadata;
mod test_metadata;

pub use metadata::METADATA;
pub use test_metadata::TEST_METADATA;
" > "$generated_dir/mod.rs"