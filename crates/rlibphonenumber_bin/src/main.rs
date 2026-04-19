use std::error::Error;

mod build_metadata;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    build_metadata::build_metadata()
}
