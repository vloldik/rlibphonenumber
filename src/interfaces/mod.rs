/// Internal phonenumber matching API used to isolate the underlying
/// implementation of the matcher and allow different implementations to be
/// swapped in easily.
pub(crate) trait MatcherApi {
  /// Returns whether the given national number (a string containing only decimal
  /// digits) matches the national number pattern defined in the given
  /// PhoneNumberDesc message.
  fn match_national_number(number: &str, number_desc: &PhoneNumberDesc, allow_prefix_match: bool) -> bool;
}