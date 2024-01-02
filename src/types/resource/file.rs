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
use crate::parsers::default::ResourceReader;
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
  pub(crate) root: &'a ResourceReader<'a>,
}

impl<'a> ResourceBase for ResourceFile<'a> {}

impl<'a> ResourceFile<'a> {
  pub(crate) fn new(index: u32, root: &'a ResourceReader<'a>) -> ResourceFile<'a> {
    Self {
      ptr: root.find_ptr(index),
      absolute_path: PathBuf::new(),
      root,
    }
  }

  pub fn name(&self) -> error::Result<String> {
    Self::internal_get_name(self.root.bytes, self.ptr, self.root.name_offset)
  }

  pub fn territory(&self) -> error::Result<Territory> {
    let raw = self
      .root
      .bytes
      .read_from_offset(self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>())
      .wrap_error_lazy(|| format!("Failed to read resource territory at {}", to_hex!(self.ptr)))?;

    Territory::from_repr(raw).ok_or_else(|| {
      Error::InvalidData(anyhow!(
        "An invalid territory was detected at {} with value {}",
        to_hex!(self.ptr),
        raw
      ))
    })
  }

  pub fn language(&self) -> error::Result<Language> {
    let raw = self
      .root
      .bytes
      .read_from_offset(self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>() * 2)
      .wrap_error_lazy(|| format!("Failed to read resource territory at {}", to_hex!(self.ptr)))?;

    Language::from_repr(raw).ok_or_else(|| {
      Error::InvalidData(anyhow!(
        "An invalid language was detected at {} with value {}",
        to_hex!(self.ptr),
        raw
      ))
    })
  }

  pub fn compression_algo(&self) -> error::Result<CompressionAlgorithm> {
    Self::internal_get_flags(self.root.bytes, self.ptr).map(CompressionAlgorithm::from)
  }

  pub fn last_modified(&self) -> error::Result<Option<DateTime<Local>>> {
    if self.root.format_version < 2 {
      return Ok(None);
    }

    let raw = self
      .root
      .bytes
      .read_from_offset::<u64>(self.ptr + mem::size_of::<u32>() * 2 + mem::size_of::<u16>() * 3)
      .wrap_error_lazy(|| {
        format!(
          "Failed to read resource last modified date at {}",
          to_hex!(self.ptr)
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
    Self::internal_get_hash(self.root.bytes, self.ptr, self.root.name_offset)
  }

  pub(crate) fn data_offset(&self) -> error::Result<u32> {
    self
      .root
      .bytes
      .read_from_offset(self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>() * 3)
      .wrap_error_lazy(|| {
        format!(
          "Failed to read resource data offset at {}",
          to_hex!(self.ptr)
        )
      })
  }

  pub(crate) fn raw_data(&self) -> error::Result<&'a [u8]> {
    let mut offset = self.root.data_offset + self.data_offset()? as usize;
    let size = self
      .root
      .bytes
      .read_from_offset::<u32>(offset)
      .wrap_error_lazy(|| format!("Failed to read resource data size at {}", to_hex!(self.ptr)))?;
    offset += mem::size_of::<u32>();

    self
      .root
      .bytes
      .get(offset..offset + size as usize)
      .ok_or_else(|| {
        Error::OutOfBounds(anyhow!(
          "Failed to read resource data at {}",
          to_hex!(self.ptr)
        ))
      })
  }
}
