/**
 * This file represents content of https://github.com/google/libphonenumber/tree/master/tools/cpp
 */

use std::{collections::BTreeMap, fs::File, io::{BufRead, BufReader}, num::ParseIntError, path::Path};

use thiserror::Error;

#[derive(Debug, Error)]
enum BuildError {
    #[error("IO error occurred: {0}")]
    IO(#[from] std::io::Error),

    #[error("Line {line_num} is too long (max is {max_len} bytes)")]
    LineTooLong { line_num: usize, max_len: usize },

    #[error("Failed to parse prefix '{prefix}': {source}")]
    PrefixParseError {
        prefix: String,
        #[source]
        source: ParseIntError,
    },
}

fn parse_prefixes(path: &str, prefixes: &mut BTreeMap<i32, String>) -> Result<(), BuildError> {
    prefixes.clear();

    let input = File::open(path)?; 
    const MAX_LINE_LENGTH: usize = 2 * 1024;
    
    let mut reader = BufReader::new(input);
    let mut line_buffer = String::with_capacity(MAX_LINE_LENGTH);
    let mut line_number = 0;

    loop {
        line_number += 1;
        line_buffer.clear();

        let bytes_read = reader.read_line(&mut line_buffer)?;
        if bytes_read == 0 {
            break;
        }

        if !line_buffer.ends_with('\n') {
             return Err(BuildError::LineTooLong {
                line_num: line_number,
                max_len: MAX_LINE_LENGTH,
            });
        }
        
        let line = line_buffer.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((prefix_str, desc)) = line.split_once('|') {
            if prefix_str.is_empty() {
                continue;
            }
            let prefix_code = prefix_str.parse().map_err(|e| BuildError::PrefixParseError {
                prefix: prefix_str.to_string(),
                source: e,
            })?;
            prefixes.insert(prefix_code, desc.to_string());
        }
    }

    Ok(())
}



fn main() -> Result<(), BuildError> {
    protobuf_codegen::Codegen::new()
        .pure()
        .includes(["resources"])
        .input("resources/phonemetadata.proto")
        .input("resources/phonenumber.proto")
        .cargo_out_dir("proto_gen")
        .run_from_script();
    Ok(())
}