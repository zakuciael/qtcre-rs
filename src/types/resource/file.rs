/*
 * qtcre-rs
 * Copyright (c) 2024 Krzysztof Saczuk <me@krzysztofsaczuk.pl>.
 *
 * This program is free software: you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of  MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::borrow::Cow;
use std::io::Read;
use std::mem;
use std::path::PathBuf;

use anyhow::anyhow;
use byteorder::{BigEndian, ReadBytesExt};
use chrono::{DateTime, Local, NaiveDateTime};
use educe::Educe;
use flate2::bufread::ZlibDecoder;

use crate::bytes::ReadFromOffset;
use crate::error;
use crate::error::{Error, WrapError};
use crate::readers::ResourceReader;
use crate::types::resource::base::ResourceBase;
use crate::types::{CompressionAlgorithm, Language, Territory};
use crate::utils::{to_hex, to_pretty_hex};

#[derive(Educe)]
#[educe(Debug)]
pub struct ResourceFile<'a> {
  #[educe(Debug(method = "to_pretty_hex"))]
  pub(crate) ptr: usize,
  pub(crate) absolute_path: PathBuf,
  #[educe(Debug(ignore))]
  pub(crate) reader: &'a ResourceReader<'a>,
}

impl<'a> ResourceBase for ResourceFile<'a> {}

impl<'a> ResourceFile<'a> {
  pub(crate) fn new(index: u32, reader: &'a ResourceReader<'a>) -> ResourceFile<'a> {
    Self {
      ptr: reader.find_ptr(index),
      absolute_path: PathBuf::new(),
      reader,
    }
  }

  pub fn name(&self) -> error::Result<String> {
    Self::internal_get_name(self.reader.bytes, self.ptr, self.reader.name_offset)
  }

  pub fn territory(&self) -> error::Result<Territory> {
    let offset = self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>();
    let raw = self
      .reader
      .bytes
      .read_from_offset(offset)
      .wrap_error_lazy(|| format!("Failed to read resource territory at {:#02x}", offset))?;

    Territory::from_repr(raw).ok_or_else(|| {
      Error::InvalidData(anyhow!(
        "An invalid territory was detected at {:#02x} with value {}",
        offset,
        raw
      ))
    })
  }

  pub fn language(&self) -> error::Result<Language> {
    let offset = self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>() * 2;
    let raw = self
      .reader
      .bytes
      .read_from_offset(offset)
      .wrap_error_lazy(|| format!("Failed to read resource territory at {:#02x}", offset))?;

    Language::from_repr(raw).ok_or_else(|| {
      Error::InvalidData(anyhow!(
        "An invalid language was detected at {:#02x} with value {}",
        offset,
        raw
      ))
    })
  }

  pub fn compression_algo(&self) -> error::Result<CompressionAlgorithm> {
    Self::internal_get_flags(self.reader.bytes, self.ptr).map(CompressionAlgorithm::from)
  }

  pub fn last_modified(&self) -> error::Result<Option<DateTime<Local>>> {
    if self.reader.format_version < 2 {
      return Ok(None);
    }

    let offset = self.ptr + mem::size_of::<u32>() * 2 + mem::size_of::<u16>() * 3;
    let raw = self
      .reader
      .bytes
      .read_from_offset::<u64>(offset)
      .wrap_error_lazy(|| {
        format!(
          "Failed to read resource last modified date at {:#02x}",
          offset
        )
      })?;

    Ok(
      NaiveDateTime::from_timestamp_millis(raw as i64)
        .as_ref()
        .map(NaiveDateTime::and_utc)
        .map(|utc| utc.with_timezone(&Local)),
    )
  }

  pub fn size(&self) -> error::Result<u64> {
    let data = self.raw_data()?;

    if data.is_empty() {
      return Ok(0);
    }

    Ok(match self.compression_algo()? {
      CompressionAlgorithm::None => data.len() as u64,
      CompressionAlgorithm::Zstd => zstd_safe::get_frame_content_size(data)
        .map_err(|err| {
          Error::InvalidData(anyhow!("Failed to read zstd uncompressed file size, frame is too small or it appears corrupted").context(err))
        })?
        .ok_or_else(|| {
          Error::InvalidData(anyhow!(
            "Failed to read zstd uncompressed file size, frame doesn't include content size"
          ))
        })?,
      CompressionAlgorithm::Zlib => {
        (&data[..])
          .read_u32::<BigEndian>()
          .wrap_error("Failed to read zlib uncompressed size")? as u64
      }
    })
  }

  pub fn data(&self) -> error::Result<Cow<'a, [u8]>> {
    let data = self.raw_data()?;
    let compression_algo = self.compression_algo()?;

    if data.is_empty() || compression_algo == CompressionAlgorithm::None {
      return Ok(Cow::Borrowed(data));
    }

    let data = {
      let mut buf: Vec<u8> = Vec::with_capacity(self.size()? as usize);
      match compression_algo {
        CompressionAlgorithm::Zstd => zstd_safe::decompress(&mut buf, data)
          .map_err(|err| Error::IO(anyhow!("Failed to decompress zstd file").context(err)))?,
        CompressionAlgorithm::Zlib => {
          let data = data
            .get(mem::size_of::<u32>()..)
            .ok_or_else(|| Error::OutOfBounds(anyhow!("Failed to decompress zlib file")))?;
          let mut decoder = ZlibDecoder::new(data);

          decoder
            .read_to_end(&mut buf)
            .map_err(|err| Error::IO(anyhow!("Failed to decompress zlib file").context(err)))?
        }
        _ => unreachable!(),
      };

      buf
    };

    Ok(Cow::Owned(data))
  }

  pub(crate) fn hash(&self) -> error::Result<u32> {
    Self::internal_get_hash(self.reader.bytes, self.ptr, self.reader.name_offset)
  }

  pub(crate) fn data_offset(&self) -> error::Result<u32> {
    let offset = self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>() * 3;

    self
      .reader
      .bytes
      .read_from_offset(offset)
      .wrap_error_lazy(|| format!("Failed to read resource data offset at {:#02x}", offset))
  }

  pub(crate) fn raw_data(&self) -> error::Result<&'a [u8]> {
    let mut offset = self.reader.data_offset + self.data_offset()? as usize;
    let size = self
      .reader
      .bytes
      .read_from_offset::<u32>(offset)
      .wrap_error_lazy(|| format!("Failed to read resource data size at {:#02x}", offset))?;
    offset += mem::size_of::<u32>();

    self
      .reader
      .bytes
      .get(offset..offset + size as usize)
      .ok_or_else(|| Error::OutOfBounds(anyhow!("Failed to read resource data at {:#02x}", offset)))
  }
}

#[cfg(test)]
mod tests {
  // 4 bytes - Name offset
  // 2 bytes - Flags
  // 2 bytes - Territory
  // 2 bytes - Language
  // 4 bytes - Data offset
  // [Optional] 8 bytes - Last modified date

  use crate::readers::ResourceReader;
  use crate::types::{CompressionAlgorithm, Language, ResourceFile, Territory};

  #[test]
  fn should_correctly_read_resource() {
    let bytes: &[u8] = &[
      0x00, 0x00, 0x00, 0x00, // Name offset
      0x00, 0x00, // Flags
      0x00, 0x02, // Territory
      0x00, 0x3B, // Language
      0x00, 0x00, 0x00, 0x05, // Data offset
      0x00, 0x00, 0x00, 0x00, 0x45, 0xEF, 0x51, 0x6C, // Last modified date
      0xFF, 0xFF, // Spacing
      0x00, 0x09, // Name length
      0x08, 0x2F, 0xA5, 0x07, // name hash
      0x00, 0x73, 0x00, 0x6D, 0x00, 0x61, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x2E, 0x00, 0x6A, 0x00,
      0x70, 0x00, 0x67, // "small.jpg"
      0xFF, 0xFF, // Spacing
      0x00, 0x00, 0x00, 0x00, 0x00, // Data spacing
      0x00, 0x00, 0x00, 0x0C, // Data size
      0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
      0x21, // "hello world!"
    ];

    let reader = ResourceReader::from_bytes(&bytes, 0, 24, 50, 3).expect("Failed to create reader");
    let resource = ResourceFile::new(0, &reader);

    let name = resource.name();
    let compression = resource.compression_algo();
    let territory = resource.territory();
    let language = resource.language();
    let last_modified = resource.last_modified();
    let data_offset = resource.data_offset();
    let data = resource.raw_data();

    assert!(name.is_ok());
    assert_eq!(name.unwrap(), "small.jpg");

    assert!(compression.is_ok());
    assert_eq!(compression.unwrap(), CompressionAlgorithm::None);

    assert!(territory.is_ok());
    assert_eq!(territory.unwrap(), Territory::Albania);

    assert!(language.is_ok());
    assert_eq!(language.unwrap(), Language::Japanese);

    assert!(last_modified.is_ok());
    let last_modified = last_modified.unwrap();
    assert!(last_modified.is_some());
    assert_eq!(last_modified.unwrap().timestamp_millis(), 1173311852);

    assert!(data_offset.is_ok());
    assert_eq!(data_offset.unwrap(), 0x05);

    assert!(data.is_ok());
    assert_eq!(
      data.unwrap(),
      &[0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x21]
    )
  }
}
