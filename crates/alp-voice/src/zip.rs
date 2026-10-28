//! Lightweight, self-contained ZIP archive reader and writer for `.alpvoice` packages.
//!
//! Complies with PKWARE ZIP specification using method 0 (STORED), allowing zero-dependency
//! packaging and extraction interoperable with standard `unzip` and system archive utilities.

use alp_core::CoreError;
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

const LOCAL_FILE_HEADER_SIG: u32 = 0x0403_4b50;
const CENTRAL_DIR_HEADER_SIG: u32 = 0x0201_4b50;
const END_OF_CENTRAL_DIR_SIG: u32 = 0x0605_4b50;

/// Standard IEEE 802.3 CRC-32 checksum calculation.
#[must_use]
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

struct CentralEntry {
    filename: String,
    crc: u32,
    size: u32,
    offset: u32,
}

/// Simple in-memory or stream-based ZIP writer.
pub struct ZipWriter<W: Write + Seek> {
    writer: W,
    entries: Vec<CentralEntry>,
}

impl<W: Write + Seek> ZipWriter<W> {
    /// Create a new ZIP archive writer wrapping the target output stream.
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            entries: Vec::new(),
        }
    }

    /// Add a file entry to the ZIP archive.
    pub fn add_file(&mut self, name: &str, data: &[u8]) -> Result<(), CoreError> {
        let offset = self.writer.stream_position().map_err(CoreError::Io)? as u32;

        let crc = crc32(data);
        let size = data.len() as u32;
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u16;

        // Local file header (30 bytes + name)
        self.writer
            .write_all(&LOCAL_FILE_HEADER_SIG.to_le_bytes())
            .map_err(CoreError::Io)?;
        self.writer
            .write_all(&20u16.to_le_bytes())
            .map_err(CoreError::Io)?; // version needed
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // flags
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // compression: stored
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // mod time
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // mod date
        self.writer
            .write_all(&crc.to_le_bytes())
            .map_err(CoreError::Io)?;
        self.writer
            .write_all(&size.to_le_bytes())
            .map_err(CoreError::Io)?; // compressed size
        self.writer
            .write_all(&size.to_le_bytes())
            .map_err(CoreError::Io)?; // uncompressed size
        self.writer
            .write_all(&name_len.to_le_bytes())
            .map_err(CoreError::Io)?;
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // extra field len
        self.writer.write_all(name_bytes).map_err(CoreError::Io)?;
        self.writer.write_all(data).map_err(CoreError::Io)?;

        self.entries.push(CentralEntry {
            filename: name.to_string(),
            crc,
            size,
            offset,
        });
        Ok(())
    }

    /// Finalize and close the ZIP archive, writing the central directory.
    pub fn finish(mut self) -> Result<(), CoreError> {
        let cd_offset = self.writer.stream_position().map_err(CoreError::Io)? as u32;

        let mut cd_size = 0u32;
        let entry_count = self.entries.len() as u16;

        for entry in &self.entries {
            let name_bytes = entry.filename.as_bytes();
            let name_len = name_bytes.len() as u16;
            let header_len = 46 + name_len as u32;

            self.writer
                .write_all(&CENTRAL_DIR_HEADER_SIG.to_le_bytes())
                .map_err(CoreError::Io)?;
            self.writer
                .write_all(&20u16.to_le_bytes())
                .map_err(CoreError::Io)?; // made by
            self.writer
                .write_all(&20u16.to_le_bytes())
                .map_err(CoreError::Io)?; // version needed
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // flags
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // compression
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // time
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // date
            self.writer
                .write_all(&entry.crc.to_le_bytes())
                .map_err(CoreError::Io)?;
            self.writer
                .write_all(&entry.size.to_le_bytes())
                .map_err(CoreError::Io)?;
            self.writer
                .write_all(&entry.size.to_le_bytes())
                .map_err(CoreError::Io)?;
            self.writer
                .write_all(&name_len.to_le_bytes())
                .map_err(CoreError::Io)?;
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // extra len
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // comment len
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // disk start
            self.writer
                .write_all(&0u16.to_le_bytes())
                .map_err(CoreError::Io)?; // internal attr
            self.writer
                .write_all(&0u32.to_le_bytes())
                .map_err(CoreError::Io)?; // external attr
            self.writer
                .write_all(&entry.offset.to_le_bytes())
                .map_err(CoreError::Io)?;
            self.writer.write_all(name_bytes).map_err(CoreError::Io)?;

            cd_size += header_len;
        }

        // End of central directory record (22 bytes)
        self.writer
            .write_all(&END_OF_CENTRAL_DIR_SIG.to_le_bytes())
            .map_err(CoreError::Io)?;
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // disk number
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // cd disk
        self.writer
            .write_all(&entry_count.to_le_bytes())
            .map_err(CoreError::Io)?; // entries on disk
        self.writer
            .write_all(&entry_count.to_le_bytes())
            .map_err(CoreError::Io)?; // total entries
        self.writer
            .write_all(&cd_size.to_le_bytes())
            .map_err(CoreError::Io)?;
        self.writer
            .write_all(&cd_offset.to_le_bytes())
            .map_err(CoreError::Io)?;
        self.writer
            .write_all(&0u16.to_le_bytes())
            .map_err(CoreError::Io)?; // comment len

        self.writer.flush().map_err(CoreError::Io)?;
        Ok(())
    }
}

/// Simple in-memory or stream-based ZIP reader.
pub struct ZipReader {
    files: HashMap<String, Vec<u8>>,
}

impl ZipReader {
    /// Read all entries from a raw ZIP archive byte slice.
    pub fn new(bytes: &[u8]) -> Result<Self, CoreError> {
        let mut cursor = Cursor::new(bytes);
        let mut files = HashMap::new();

        let total_len = bytes.len();
        if total_len < 22 {
            return Err(CoreError::validation("archive too small to be a ZIP"));
        }

        // Find End of Central Directory signature (search backwards from end)
        let mut eocd_pos = None;
        let search_start = total_len.saturating_sub(65536 + 22);
        for pos in (search_start..=total_len - 22).rev() {
            if bytes[pos..pos + 4] == END_OF_CENTRAL_DIR_SIG.to_le_bytes() {
                eocd_pos = Some(pos);
                break;
            }
        }

        let eocd_offset =
            eocd_pos.ok_or_else(|| CoreError::validation("missing ZIP EOCD record"))?;
        cursor
            .seek(SeekFrom::Start((eocd_offset + 10) as u64))
            .map_err(CoreError::Io)?;

        let mut u16_buf = [0u8; 2];
        cursor.read_exact(&mut u16_buf).map_err(CoreError::Io)?;
        let entry_count = u16::from_le_bytes(u16_buf);

        let mut u32_buf = [0u8; 4];
        cursor
            .seek(SeekFrom::Start((eocd_offset + 16) as u64))
            .map_err(CoreError::Io)?;
        cursor.read_exact(&mut u32_buf).map_err(CoreError::Io)?;
        let cd_offset = u32::from_le_bytes(u32_buf);

        cursor
            .seek(SeekFrom::Start(u64::from(cd_offset)))
            .map_err(CoreError::Io)?;

        for _ in 0..entry_count {
            let mut sig_buf = [0u8; 4];
            cursor.read_exact(&mut sig_buf).map_err(CoreError::Io)?;
            if sig_buf != CENTRAL_DIR_HEADER_SIG.to_le_bytes() {
                return Err(CoreError::validation("corrupt central directory header"));
            }

            // Read central directory record
            let mut header_buf = [0u8; 42];
            cursor.read_exact(&mut header_buf).map_err(CoreError::Io)?;

            let compression = u16::from_le_bytes([header_buf[6], header_buf[7]]);
            let crc = u32::from_le_bytes([
                header_buf[12],
                header_buf[13],
                header_buf[14],
                header_buf[15],
            ]);
            let size = u32::from_le_bytes([
                header_buf[16],
                header_buf[17],
                header_buf[18],
                header_buf[19],
            ]) as usize;
            let name_len = u16::from_le_bytes([header_buf[24], header_buf[25]]) as usize;
            let extra_len = u16::from_le_bytes([header_buf[26], header_buf[27]]) as usize;
            let comment_len = u16::from_le_bytes([header_buf[28], header_buf[29]]) as usize;
            let local_offset = u32::from_le_bytes([
                header_buf[38],
                header_buf[39],
                header_buf[40],
                header_buf[41],
            ]) as usize;

            let mut name_bytes = vec![0u8; name_len];
            cursor.read_exact(&mut name_bytes).map_err(CoreError::Io)?;
            let filename = String::from_utf8_lossy(&name_bytes).to_string();

            // Skip extra and comment
            cursor
                .seek(SeekFrom::Current((extra_len + comment_len) as i64))
                .map_err(CoreError::Io)?;

            if compression != 0 {
                return Err(CoreError::validation(format!(
                    "unsupported compression method {compression} for {filename}"
                )));
            }

            // Save current CD cursor pos
            let next_cd_pos = cursor.stream_position().map_err(CoreError::Io)?;

            // Read data from local file header
            let mut local_head = [0u8; 30];
            cursor
                .seek(SeekFrom::Start(local_offset as u64))
                .map_err(CoreError::Io)?;
            cursor.read_exact(&mut local_head).map_err(CoreError::Io)?;

            let loc_name_len = u16::from_le_bytes([local_head[26], local_head[27]]) as usize;
            let loc_extra_len = u16::from_le_bytes([local_head[28], local_head[29]]) as usize;
            let data_offset = local_offset + 30 + loc_name_len + loc_extra_len;

            cursor
                .seek(SeekFrom::Start(data_offset as u64))
                .map_err(CoreError::Io)?;
            let mut data = vec![0u8; size];
            cursor.read_exact(&mut data).map_err(CoreError::Io)?;

            if crc32(&data) != crc {
                return Err(CoreError::validation(format!(
                    "CRC mismatch for {filename}"
                )));
            }

            files.insert(filename, data);

            // Restore cursor to continue CD reading
            cursor
                .seek(SeekFrom::Start(next_cd_pos))
                .map_err(CoreError::Io)?;
        }

        Ok(Self { files })
    }

    /// Read file content by exact name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(Vec::as_slice)
    }

    /// Return all filenames present in the archive.
    #[must_use]
    pub fn filenames(&self) -> Vec<&str> {
        self.files.keys().map(String::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_standard() {
        let text = b"123456789";
        // Standard CRC-32 for "123456789" is 0xcbf43926
        assert_eq!(crc32(text), 0xcbf4_3926);
    }

    #[test]
    fn zip_write_and_read_round_trip() {
        let mut buf = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut buf);
        writer
            .add_file("hello.txt", b"Hello, Audio Labs Pro!")
            .expect("add file");
        writer
            .add_file("test.json", br#"{"status": "ok"}"#)
            .expect("add json");
        writer.finish().expect("finish");

        let bytes = buf.into_inner();
        let reader = ZipReader::new(&bytes).expect("reader");

        assert_eq!(
            reader.get("hello.txt"),
            Some(&b"Hello, Audio Labs Pro!"[..])
        );
        assert_eq!(reader.get("test.json"), Some(&br#"{"status": "ok"}"#[..]));
        assert_eq!(reader.get("missing.txt"), None);
    }
}
