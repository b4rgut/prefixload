use crate::error::Result;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

// --- Helper for Streaming Memory-Efficient Reads ---

/// A reader for a specific section (part) of a file.
/// It implements `Read` by using thread-safe positioned reads on the underlying file,
/// allowing for memory-efficient streaming of file parts.
struct FilePartReader<'a> {
    file: &'a File,
    cursor: u64,
    end: u64,
}

impl<'a> Read for FilePartReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cursor >= self.end {
            return Ok(0);
        }

        let max_read = (self.end - self.cursor) as usize;
        let read_buf = if buf.len() > max_read {
            &mut buf[..max_read]
        } else {
            buf
        };

        #[cfg(unix)]
        let bytes_read = {
            use std::os::unix::fs::FileExt;
            self.file.read_at(read_buf, self.cursor)?
        };

        #[cfg(windows)]
        let bytes_read = {
            use std::os::windows::fs::FileExt;
            self.file.seek_read(read_buf, self.cursor)?
        };

        #[cfg(not(any(unix, windows)))]
        let bytes_read: usize = {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Positioned reads not supported on this platform",
            ));
        };

        self.cursor += bytes_read as u64;
        Ok(bytes_read)
    }
}

/// Calculates the S3 ETag for a file.
pub fn calculate_s3_etag<P: AsRef<Path> + Sync>(file_path: P, part_size: u64) -> Result<String> {
    let metadata = std::fs::metadata(file_path.as_ref())?;
    let file_size = metadata.len();

    if file_size == 0 {
        return Ok(format!("{:x}", md5::compute(b"")));
    }

    if file_size <= part_size {
        let mut file = File::open(file_path.as_ref())?;
        let mut md5_context = md5::Context::new();
        io::copy(&mut file, &mut md5_context)?;
        let digest = md5_context.finalize();
        Ok(format!("{:x}", digest))
    } else {
        let file = File::open(file_path.as_ref())?;
        let num_parts = (file_size + part_size - 1) / part_size;

        // Collect the md5::Digest results directly to avoid intermediate Vec<u8> allocations.
        let part_digests: Result<Vec<md5::Digest>> = (0..num_parts)
            .into_par_iter()
            .map(|part_num| -> Result<md5::Digest> {
                let offset = part_num * part_size;
                let bytes_to_read = std::cmp::min(part_size, file_size - offset);

                let mut md5_context = md5::Context::new();

                let mut part_reader = FilePartReader {
                    file: &file,
                    cursor: offset,
                    end: offset + bytes_to_read,
                };

                if let Err(e) = io::copy(&mut part_reader, &mut md5_context) {
                    if e.kind() == io::ErrorKind::Unsupported {
                        let mut f = File::open(file_path.as_ref())?;
                        f.seek(SeekFrom::Start(offset))?;
                        let mut limited_reader = f.take(bytes_to_read);
                        io::copy(&mut limited_reader, &mut md5_context)?;
                    } else {
                        return Err(e.into());
                    }
                }

                Ok(md5_context.finalize())
            })
            .collect();

        let digests = part_digests?;
        let mut combined_hashes = Vec::with_capacity(digests.len() * 16);
        for digest in digests {
            combined_hashes.extend_from_slice(&digest.0);
        }

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
        let etag = calculate_s3_etag(file.path(), 5 * MB).unwrap();
        assert_eq!(etag, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_etag_single_part_small() {
        let mut file = NamedTempFile::new().unwrap();
        let content = b"hello world";
        file.write_all(content).unwrap();
        let etag = calculate_s3_etag(file.path(), 5 * MB).unwrap();
        let expected_etag = format!("{:x}", md5::compute(content));
        assert_eq!(etag, expected_etag);
    }

    #[test]
    fn test_etag_single_part_large_streaming() {
        let file_size = 6 * MB;
        let part_size = 10 * MB;
        let mut file = NamedTempFile::new().unwrap();
        let content = vec![b'b'; file_size as usize];
        file.write_all(&content).unwrap();
        let etag = calculate_s3_etag(file.path(), part_size).unwrap();
        let expected_etag = format!("{:x}", md5::compute(&content));
        assert_eq!(etag, expected_etag);
    }

    #[test]
    fn test_etag_multi_part() {
        let part_size = 1 * MB;
        let file_size = (2.5 * MB as f64) as usize;
        let mut file = NamedTempFile::new().unwrap();
        let content = vec![b'a'; file_size];
        file.write_all(&content).unwrap();

        let part1_md5 = md5::compute(&content[0..(1 * MB as usize)]);
        let part2_md5 = md5::compute(&content[(1 * MB as usize)..(2 * MB as usize)]);
        let part3_md5 = md5::compute(&content[(2 * MB as usize)..]);

        let mut combined_md5s = Vec::new();
        combined_md5s.extend_from_slice(&part1_md5.0);
        combined_md5s.extend_from_slice(&part2_md5.0);
        combined_md5s.extend_from_slice(&part3_md5.0);

        let final_md5 = md5::compute(&combined_md5s);
        let expected_etag = format!("{:x}-3", final_md5);

        let etag = calculate_s3_etag(file.path(), part_size).unwrap();
        assert_eq!(etag, expected_etag);
    }
}
