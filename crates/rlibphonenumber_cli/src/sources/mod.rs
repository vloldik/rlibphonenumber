mod find_numbers;

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone)]
pub enum Source {
    File(PathBuf),
    Http(Url),
    Ssh {
        user_host: String,
        path: String,
    },
    Git {
        repo: String,
        file_path: String,
        branch: Option<String>,
    },
}

#[derive(Debug, Clone, Error)]
pub enum SourceParseError {
    #[error("Failed to extract a valid file path from the URL")]
    InvalidFilePath,
}

impl FromStr for Source {
    type Err = SourceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Attempt to parse as a standard URL
        if let Ok(url) = Url::parse(s) {
            match url.scheme() {
                "http" | "https" => return Ok(Source::Http(url)),
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|_| SourceParseError::InvalidFilePath)?;
                    return Ok(Source::File(path));
                }
                "ssh" => {
                    let host = url.host_str().unwrap_or_default();
                    let user = url.username();
                    let user_host = if user.is_empty() {
                        host.to_string()
                    } else {
                        format!("{}@{}", user, host)
                    };
                    return Ok(Source::Ssh {
                        user_host,
                        path: url.path().to_string(),
                    });
                }
                "git" => {
                    let mut file_path = String::new();
                    let mut branch = None;

                    for (key, value) in url.query_pairs() {
                        if key == "file" {
                            file_path = value.into_owned();
                        } else if key == "branch" {
                            branch = Some(value.into_owned());
                        }
                    }

                    let mut repo_url = url.clone();
                    repo_url.set_query(None);

                    return Ok(Source::Git {
                        repo: repo_url.into(),
                        file_path,
                        branch,
                    });
                }
                _ => {} // Fallthrough for unknown schemes
            }
        }

        // Fallback for SCP-like SSH strings (e.g., user@host:/path)
        if s.contains('@') && s.contains(':') && !s.starts_with("http") {
            if let Some((user_host, path)) = s.split_once(':') {
                return Ok(Source::Ssh {
                    user_host: user_host.to_string(),
                    path: path.to_string(),
                });
            }
        }

        // Ultimate fallback: treat as a local file path
        Ok(Source::File(PathBuf::from(s)))
    }
}

#[derive(Debug, Error)]
pub enum SourceReadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("SSH command failed: {0}")]
    Ssh(String),

    #[error("Git command failed: {0}")]
    Git(String),
}

impl Source {
    /// Reads the content from the given source.
    /// Returns a dynamically dispatched `BufRead`. Streams the data where natively possible,
    /// or downloads it securely into an anonymous temporary file that is automatically
    /// deleted when dropped.
    pub fn read(&self) -> Result<Box<dyn BufRead>, SourceReadError> {
        match self {
            Source::File(path) => {
                let file = File::open(path)?;
                Ok(Box::new(BufReader::new(file)))
            }

            Source::Http(url) => {
                let response = reqwest::blocking::get(url.clone())?.error_for_status()?;
                Ok(Box::new(BufReader::new(response)))
            }

            Source::Ssh { user_host, path } => {
                // Anonymous temp file. Auto-deletes on handle drop.
                let mut temp_file = tempfile::tempfile()?;

                let child = Command::new("ssh")
                    .arg(user_host)
                    .arg("cat")
                    .arg(path)
                    .stdout(temp_file.try_clone()?) // Stream directly to temp file, bypassing RAM
                    .stderr(Stdio::piped())
                    .spawn()?;

                let output = child.wait_with_output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                    return Err(SourceReadError::Ssh(stderr));
                }

                temp_file.seek(SeekFrom::Start(0))?;
                Ok(Box::new(BufReader::new(temp_file)))
            }

            Source::Git {
                repo,
                file_path,
                branch,
            } => {
                let mut temp_file = tempfile::tempfile()?;
                let branch_name = branch.as_deref().unwrap_or("HEAD");

                let archive_cmd = format!(
                    "git archive --remote='{}' '{}' '{}' | tar -xO",
                    repo, branch_name, file_path
                );

                let child = Command::new("sh")
                    .arg("-c")
                    .arg(&archive_cmd)
                    .stdout(temp_file.try_clone()?) // Stream directly to temp file
                    .stderr(Stdio::piped())
                    .spawn()?;

                let output = child.wait_with_output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                    return Err(SourceReadError::Git(stderr));
                }

                temp_file.seek(SeekFrom::Start(0))?;
                Ok(Box::new(BufReader::new(temp_file)))
            }
        }
    }

    /// Extracts a reliable file name from the source string to be used for validation checks
    pub fn file_name(&self) -> Option<String> {
        match self {
            Source::File(path) => path.file_name().map(|s| s.to_string_lossy().into_owned()),
            Source::Http(url) => url
                .path_segments()
                .and_then(|seg| seg.last())
                .map(|s| s.to_string()),
            Source::Ssh { path, .. } => path.split('/').last().map(|s| s.to_string()),
            Source::Git { file_path, .. } => file_path.split('/').last().map(|s| s.to_string()),
        }
    }
}
