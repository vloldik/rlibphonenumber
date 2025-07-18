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

use crate::generated::proto::phonemetadata::PhoneNumberDesc;

/// Internal phonenumber matching API used to isolate the underlying
/// implementation of the matcher and allow different implementations to be
/// swapped in easily.
pub(crate) trait MatcherApi: Send + Sync {
  /// Returns whether the given national number (a string containing only decimal
  /// digits) matches the national number pattern defined in the given
  /// PhoneNumberDesc message.
  fn match_national_number(&self, number: &str, number_desc: &PhoneNumberDesc, allow_prefix_match: bool) -> bool;
}