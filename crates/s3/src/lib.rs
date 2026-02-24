#![deny(missing_docs)]

//! S3 helper functions.
//!
//! This crate provides utilities for interacting with Amazon S3-compatible
//! object storage services.
//!
//! ## Features
//!
//! - Object upload/download operations
//! - Pre-signed URL generation
//! - Bucket management utilities
//! - Multipart upload support
//!
//! ## Usage
//!
//! \```ignore
//! use qm_s3::S3Client;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = S3Client::new("bucket-name").await?;
//!     client.put_object("key", data).await?;
//!     Ok(())
//! }
//! \```

/// Adds two numbers together.
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
