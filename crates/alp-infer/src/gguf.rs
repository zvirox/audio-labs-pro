//! A small GGUF reader used to validate heavy-model installations.
//!
//! The heavy clone tier ships as GGUF files. Before the runtime is loaded, the
//! engine checks that the file really is a GGUF container and that it carries
//! the architecture the engine expects — a wrong or truncated file is then
//! reported as such instead of surfacing later as a confusing tensor error.
//!
//! Only the header is parsed: the magic, the version, the tensor and metadata
//! counts, and the metadata key/value pairs. Tensor payloads are never read.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Errors produced while reading a GGUF header.
#[derive(Debug, thiserror::Error)]
pub enum GgufError {
    /// The file could not be opened or read.
    #[error("reading {path}: {source}")]
    Io {
        /// The file that could not be read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The file does not start with the GGUF magic bytes.
    #[error("{path} is not a GGUF file (bad magic bytes)")]
    NotGguf {
        /// The file that failed the check.
        path: String,
    },
    /// The GGUF version is newer than this reader understands.
    #[error("{path} uses GGUF version {version}, which this build cannot read")]
    UnsupportedVersion {
        /// The file that failed the check.
        path: String,
        /// The version found in the file.
        version: u32,
    },
    /// A metadata value used a type this reader does not know.
    #[error("{path} contains an unsupported GGUF value type {value_type}")]
    UnknownValueType {
        /// The file that failed the check.
        path: String,
        /// The unknown type tag.
        value_type: u32,
    },
    /// The header ended before the metadata was complete.
    #[error("{path} has a truncated GGUF header")]
    Truncated {
        /// The file that failed the check.
        path: String,
    },
}

/// GGUF value type tags.
mod value_type {
    /// Unsigned 8-bit integer.
    pub const U8: u32 = 0;
    /// Signed 8-bit integer.
    pub const I8: u32 = 1;
    /// Unsigned 16-bit integer.
    pub const U16: u32 = 2;
    /// Signed 16-bit integer.
    pub const I16: u32 = 3;
    /// Unsigned 32-bit integer.
    pub const U32: u32 = 4;
    /// Signed 32-bit integer.
    pub const I32: u32 = 5;
    /// 32-bit float.
    pub const F32: u32 = 6;
    /// Boolean.
    pub const BOOL: u32 = 7;
    /// Length-prefixed UTF-8 string.
    pub const STRING: u32 = 8;
    /// Array of a single value type.
    pub const ARRAY: u32 = 9;
    /// Unsigned 64-bit integer.
    pub const U64: u32 = 10;
    /// Signed 64-bit integer.
    pub const I64: u32 = 11;
    /// 64-bit float.
    pub const F64: u32 = 12;
}

/// The metadata of a GGUF file, limited to what the installer needs.
#[derive(Debug, Clone, PartialEq)]
pub struct GgufHeader {
    /// The container version (2 or 3).
    pub version: u32,
    /// Number of tensors the file declares.
    pub tensor_count: u64,
    /// Number of metadata entries the file declares.
    pub metadata_count: u64,
    /// The metadata entries, in file order.
    pub metadata: Vec<(String, MetadataValue)>,
}

impl GgufHeader {
    /// Look up a metadata value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.metadata
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The value of `general.architecture` as a string, when present.
    #[must_use]
    pub fn architecture(&self) -> Option<&str> {
        match self.get("general.architecture") {
            Some(MetadataValue::String(value)) => Some(value),
            _ => None,
        }
    }
}

/// A GGUF metadata value, reduced to the types the installer inspects.
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// Unsigned integer.
    U64(u64),
    /// Signed integer.
    I64(i64),
    /// 32-bit float.
    F32(f32),
    /// Boolean flag.
    Bool(bool),
    /// UTF-8 string.
    String(String),
    /// An array; large arrays are summarized by their length.
    Array(usize),
}

/// Read the header of a GGUF file.
pub fn read_header(path: &Path) -> Result<GgufHeader, GgufError> {
    let display = path.display().to_string();
    let file = File::open(path).map_err(|source| GgufError::Io {
        path: display.clone(),
        source,
    })?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 4];
    reader
        .read_exact(&mut magic)
        .map_err(|_| GgufError::Truncated {
            path: display.clone(),
        })?;
    if &magic != b"GGUF" {
        return Err(GgufError::NotGguf { path: display });
    }

    let version = read_u32(&mut reader, &display)?;
    if version > 3 {
        return Err(GgufError::UnsupportedVersion {
            path: display,
            version,
        });
    }
    let tensor_count = read_u64(&mut reader, &display)?;
    let metadata_count = read_u64(&mut reader, &display)?;

    let mut metadata = Vec::with_capacity(metadata_count.min(4096) as usize);
    for _ in 0..metadata_count {
        let key = read_string(&mut reader, &display)?;
        let value = read_value(&mut reader, &display)?;
        metadata.push((key, value));
    }

    Ok(GgufHeader {
        version,
        tensor_count,
        metadata_count,
        metadata,
    })
}

fn read_exact(reader: &mut impl Read, path: &str, count: usize) -> Result<Vec<u8>, GgufError> {
    let mut buffer = vec![0u8; count];
    reader
        .read_exact(&mut buffer)
        .map_err(|_| GgufError::Truncated {
            path: path.to_owned(),
        })?;
    Ok(buffer)
}

fn read_u32(reader: &mut impl Read, path: &str) -> Result<u32, GgufError> {
    let bytes = read_exact(reader, path, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(reader: &mut impl Read, path: &str) -> Result<u64, GgufError> {
    let bytes = read_exact(reader, path, 8)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn read_string(reader: &mut impl Read, path: &str) -> Result<String, GgufError> {
    let length = read_u64(reader, path)? as usize;
    // Guard against a corrupt length claiming gigabytes of header.
    if length > 64 * 1024 * 1024 {
        return Err(GgufError::Truncated {
            path: path.to_owned(),
        });
    }
    let bytes = read_exact(reader, path, length)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_value(reader: &mut impl Read, path: &str) -> Result<MetadataValue, GgufError> {
    let value_type = read_u32(reader, path)?;
    match value_type {
        value_type::U8 | value_type::BOOL => {
            let bytes = read_exact(reader, path, 1)?;
            Ok(if value_type == value_type::BOOL {
                MetadataValue::Bool(bytes[0] != 0)
            } else {
                MetadataValue::U64(u64::from(bytes[0]))
            })
        }
        value_type::I8 => {
            let bytes = read_exact(reader, path, 1)?;
            Ok(MetadataValue::I64(i64::from(bytes[0] as i8)))
        }
        value_type::U16 => {
            let bytes = read_exact(reader, path, 2)?;
            Ok(MetadataValue::U64(u64::from(u16::from_le_bytes([
                bytes[0], bytes[1],
            ]))))
        }
        value_type::I16 => {
            let bytes = read_exact(reader, path, 2)?;
            Ok(MetadataValue::I64(i64::from(i16::from_le_bytes([
                bytes[0], bytes[1],
            ]))))
        }
        value_type::U32 => Ok(MetadataValue::U64(u64::from(read_u32(reader, path)?))),
        value_type::I32 => {
            let bytes = read_exact(reader, path, 4)?;
            Ok(MetadataValue::I64(i64::from(i32::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ]))))
        }
        value_type::F32 => {
            let bytes = read_exact(reader, path, 4)?;
            Ok(MetadataValue::F32(f32::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ])))
        }
        value_type::F64 => {
            let bytes = read_exact(reader, path, 8)?;
            Ok(MetadataValue::F32(f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]) as f32))
        }
        value_type::U64 => Ok(MetadataValue::U64(read_u64(reader, path)?)),
        value_type::I64 => {
            let bytes = read_exact(reader, path, 8)?;
            Ok(MetadataValue::I64(i64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ])))
        }
        value_type::STRING => Ok(MetadataValue::String(read_string(reader, path)?)),
        value_type::ARRAY => {
            let element_type = read_u32(reader, path)?;
            let count = read_u64(reader, path)?;
            // The element payload is skipped: the installer never inspects
            // array contents, and a vocabulary can hold 150 000 entries.
            for _ in 0..count {
                skip_value(reader, path, element_type)?;
            }
            Ok(MetadataValue::Array(count as usize))
        }
        other => Err(GgufError::UnknownValueType {
            path: path.to_owned(),
            value_type: other,
        }),
    }
}

fn skip_value(reader: &mut impl Read, path: &str, value_type: u32) -> Result<(), GgufError> {
    match value_type {
        value_type::U8 | value_type::I8 | value_type::BOOL => {
            read_exact(reader, path, 1)?;
        }
        value_type::U16 | value_type::I16 => {
            read_exact(reader, path, 2)?;
        }
        value_type::U32 | value_type::I32 | value_type::F32 => {
            read_exact(reader, path, 4)?;
        }
        value_type::U64 | value_type::I64 | value_type::F64 => {
            read_exact(reader, path, 8)?;
        }
        value_type::STRING => {
            read_string(reader, path)?;
        }
        value_type::ARRAY => {
            let element_type = read_u32(reader, path)?;
            let count = read_u64(reader, path)?;
            for _ in 0..count {
                skip_value(reader, path, element_type)?;
            }
        }
        other => {
            return Err(GgufError::UnknownValueType {
                path: path.to_owned(),
                value_type: other,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal GGUF v3 header with one string metadata entry.
    fn write_gguf(path: &Path, architecture: &str) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes()); // tensor count
        bytes.extend_from_slice(&1u64.to_le_bytes()); // metadata count
        let key = b"general.architecture";
        bytes.extend_from_slice(&(key.len() as u64).to_le_bytes());
        bytes.extend_from_slice(key);
        bytes.extend_from_slice(&value_type::STRING.to_le_bytes());
        bytes.extend_from_slice(&(architecture.len() as u64).to_le_bytes());
        bytes.extend_from_slice(architecture.as_bytes());
        std::fs::write(path, bytes).expect("write fixture");
    }

    #[test]
    fn the_architecture_is_read_from_the_header() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model.gguf");
        write_gguf(&path, "qwen3-tts");

        let header = read_header(&path).expect("read header");
        assert_eq!(header.version, 3);
        assert_eq!(header.architecture(), Some("qwen3-tts"));
        assert_eq!(header.metadata_count, 1);
    }

    #[test]
    fn a_non_gguf_file_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("not.gguf");
        std::fs::write(&path, b"NOPE-not-a-gguf-file").expect("write");

        let error = read_header(&path).expect_err("must reject");
        assert!(matches!(error, GgufError::NotGguf { .. }), "{error}");
    }

    #[test]
    fn a_truncated_header_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("short.gguf");
        std::fs::write(&path, b"GGUF\x03\x00").expect("write");

        let error = read_header(&path).expect_err("must reject");
        assert!(matches!(error, GgufError::Truncated { .. }), "{error}");
    }

    #[test]
    fn a_missing_file_reports_an_io_error() {
        let error = read_header(Path::new("/nonexistent/model.gguf")).expect_err("must reject");
        assert!(matches!(error, GgufError::Io { .. }), "{error}");
    }
}
