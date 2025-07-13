#!/bin/bash

filedir="./$(dirname "$0")"
javadir="$filedir/../java"
project_home="$filedir/../.."
generated_dir="$project_home/src/generated/metadata"
echo $generated_dir

resources_dir="$project_home/resources"
rust_build_jar="$javadir/rust-build/target/rust-build-1.0-SNAPSHOT-jar-with-dependencies.jar"

copyright_header="\
// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 The Kashin Vladislav (Rust adaptation author)
//
// Licensed under the Apache License, Version 2.0 (the \"License\");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an \"AS IS\" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
"

skip_install=false

# Loop through all the command-line arguments
for arg in "$@"
do
    if [ "$arg" == "--skip-install" ]
    then
        skip_install=true
        # You can break the loop once the flag is found if you don't need to process further arguments
        break
    fi
done

if [[ $skip_install == false ]]; then
    mvn -f "$javadir/pom.xml" install
fi
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

# generate test metadata
generate "PhoneNumberMetadataForTesting.xml" "test_metadata" "metadata" "TEST_METADATA"

# remove unnecessary nesting with pub use
echo "\
$copyright_header

mod metadata;

// use only in test case 
#[cfg(test)]
mod test_metadata;

pub use metadata::METADATA;
#[cfg(test)]
pub use test_metadata::TEST_METADATA;
" > "$generated_dir/mod.rs"