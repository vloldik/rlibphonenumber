#[cfg(feature = "regex")]
pub use regex::{Error, Match, Regex};

#[cfg(all(feature = "lite", not(feature = "regex")))]
pub use regex_lite::{Error, Match, Regex};

#[cfg(not(any(feature = "regex", feature = "lite")))]
compile_error!("libphonenumber: You must enable either the 'regex' or 'lite' feature.");
