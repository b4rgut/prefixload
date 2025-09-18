use crate::error::Result;
use rayon::prelude::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Calculates the S3 ETag for a file, which is a standard MD5 hash for single-part
/// uploads or a hash of concatenated part-hashes for multipart uploads.
///
/// This function reads the file in parallel chunks to speed up the process for large files.
///
/// # Arguments
///
/// * `file_path` - The path to the file.
/// * `part_size` - The size of each part in bytes for multipart calculation.
///
/// # Returns
///
/// A `Result` containing the ETag as a string (e.g., "d41d8cd98f00b204e9800998ecf8427e" or
/// "a1b2c3d4...-N" for multipart) or a `PrefixloadError`.
pub fn calculate_s3_etag(file_path: PathBuf, part_size: u64) -> Result<String> {
    let file = File::open(&file_path)?;
    let file_size = file.metadata()?.len();

    // Handle empty file separately as it has a standard ETag.
    if file_size == 0 {
        return Ok(format!("{:x}", md5::compute(b"")));
    }

    // If the file is smaller than the part size, calculate a simple MD5 hash.
    if file_size <= part_size {
        let mut reader = File::open(&file_path)?;
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let digest = md5::compute(&buffer);
        Ok(format!("{:x}", digest))
    } else {
        // For larger files, calculate the ETag based on MD5 hashes of its parts.
        let num_parts = (file_size + part_size - 1) / part_size;

        // Process each part in parallel using rayon.
        let part_hashes: Result<Vec<Vec<u8>>> = (0..num_parts)
            .into_par_iter()
            .map(|part_num| -> Result<Vec<u8>> {
                let mut file = File::open(&file_path)?;
                let offset = part_num * part_size;
                let bytes_to_read = std::cmp::min(part_size, file_size - offset);

                let mut buffer = vec![0; bytes_to_read as usize];
                file.seek(SeekFrom::Start(offset))?;
                file.read_exact(&mut buffer)?;

                let digest = md5::compute(&buffer);
                Ok(digest.0.to_vec())
            })
            .collect(); // Collect results, stopping on the first error.

        // Concatenate the individual part hashes.
        let combined_hashes: Vec<u8> = part_hashes?.into_iter().flatten().collect();

        // Compute the final MD5 hash of the concatenated hashes.
        let final_digest = md5::compute(&combined_hashes);
        Ok(format!("{:x}-{}", final_digest, num_parts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const MB: u64 = 1024 * 1024;

    #[test]
    fn test_etag_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let etag = calculate_s3_etag(file.path().to_path_buf(), 5 * MB).unwrap();
        // ETag for an empty file is the MD5 of an empty string.
        assert_eq!(etag, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_etag_single_part() {
        let mut file = NamedTempFile::new().unwrap();
        let content = b"hello world";
        file.write_all(content).unwrap();

        let etag = calculate_s3_etag(file.path().to_path_buf(), 5 * MB).unwrap();

        // ETag for a single-part upload is just the MD5 of the content.
        let expected_etag = format!("{:x}", md5::compute(content));
        assert_eq!(etag, expected_etag);
    }

    #[test]
    fn test_etag_multi_part() {
        let part_size = 1 * MB;
        // Create a file with 2.5MB to have 3 parts.
        let file_size = (2.5 * MB as f64) as usize;
        let mut file = NamedTempFile::new().unwrap();

        // Fill the file with predictable content.
        let content = vec![b'a'; file_size];
        file.write_all(&content).unwrap();

        // Calculate the expected ETag manually.
        let part1_md5 = md5::compute(&content[0..(1 * MB as usize)]);
        let part2_md5 = md5::compute(&content[(1 * MB as usize)..(2 * MB as usize)]);
        let part3_md5 = md5::compute(&content[(2 * MB as usize)..]);

        let mut combined_md5s = Vec::new();
        combined_md5s.extend_from_slice(&part1_md5.0);
        combined_md5s.extend_from_slice(&part2_md5.0);
        combined_md5s.extend_from_slice(&part3_md5.0);

        let final_md5 = md5::compute(&combined_md5s);
        let expected_etag = format!("{:x}-3", final_md5);

        // Calculate ETag using the function.
        let etag = calculate_s3_etag(file.path().to_path_buf(), part_size).unwrap();

        assert_eq!(etag, expected_etag);
    }
}
