mod metadata;
mod test_metadata;

pub use metadata::METADATA;
// use only in test case 
#[cfg(test)]
pub use test_metadata::TEST_METADATA;

