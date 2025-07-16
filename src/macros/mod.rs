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

// std::borrow::Cow
// std::option::Option

/// This macro extracts owned value from cow
/// but if cow is borrowed it returns default given value
/// 
/// it's helpful when function returns `Cow<'_, T>` as result,
/// where `Cow::Borrowed` option marks that value was not modified
/// and we can use owned original instead of copying it.
macro_rules! owned_from_cow_or {
    ($getcow:expr, $default:expr) => {{
        if let std::borrow::Cow::Owned(s) = $getcow {
            s
        } else {
            $default
        }
    }};
}

pub(crate) use owned_from_cow_or;

