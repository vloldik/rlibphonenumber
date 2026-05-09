export const LOCK_FILE = "libphonenumber-version.lock"
export const RE2_DEFAULT = "2022-12-01"
export const RUST_IMAGE = "rust:1.93.0-trixie"
export const JDK_IMAGE = "eclipse-temurin:21.0.10_7-jdk-noble"
export const JAR = "tools/java/rust-build/target/rust-build-1.0-SNAPSHOT-jar-with-dependencies.jar"

export const COPYRIGHT_HEADER = `\
// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 Kashin Vladislav (Rust adaptation author)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
`

export const RUST_MODULE_CONTENT = `\
${COPYRIGHT_HEADER}

#[cfg(feature = "builtin_metadata")]
mod alternate_formats;

#[allow(clippy::module_inception)]
#[cfg(feature = "builtin_metadata")]
mod metadata;

#[cfg(test)]
mod test_metadata;

#[cfg(feature = "builtin_metadata")]
pub use alternate_formats::ALTERNATE_FORMATS_METADATA;
#[cfg(feature = "builtin_metadata")]
pub use metadata::METADATA;

#[cfg(test)]
pub use test_metadata::TEST_METADATA;
`
