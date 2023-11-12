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

use std::io::{Cursor, Seek, SeekFrom};
use std::mem;

use byteorder::{BigEndian, ReadBytesExt};

use crate::bytes::ReadFromOffset;
use crate::error;
use crate::error::WrapError;

pub(super) trait ResourceBase {
  fn internal_get_name(bytes: &[u8], ptr: usize, name_offset: usize) -> error::Result<String> {
    let mut reader = {
      let offset = {
        let offset = Self::internal_get_name_offset(bytes, ptr)?;
        name_offset as u64 + offset as u64
      };
      let mut reader = Cursor::new(bytes);

      reader
        .seek(SeekFrom::Start(offset))
        .wrap_error_lazy(|| format!("Failed to seek to the name table at {:#02x}", offset))?;
      reader
    };

    let length = reader.read_u16::<BigEndian>().wrap_error_lazy(|| {
      format!(
        "Failed to read resource name length at {:#02x}",
        reader.position()
      )
    })?;

    reader
      .seek(SeekFrom::Current(mem::size_of::<u32>() as i64))
      .wrap_error_lazy(|| {
        format!(
          "Failed to read resource name hash at {:#02x}",
          reader.position()
        )
      })?;

    let pos = reader.position();
    let buf = {
      let mut buf = vec![0u16; length as usize];
      reader
        .read_u16_into::<BigEndian>(&mut buf)
        .wrap_error_lazy(|| format!("Failed to read resource name at {:#02x}", pos))?;

      buf
    };

    String::from_utf16(&buf)
      .wrap_error_lazy(|| format!("Failed to parse resource name at {:#02x}", pos))
  }

  fn internal_get_hash(bytes: &[u8], ptr: usize, name_offset: usize) -> error::Result<u32> {
    let offset = {
      let resource_name_offset = Self::internal_get_name_offset(bytes, ptr)?;
      name_offset + resource_name_offset as usize + mem::size_of::<u16>()
    };

    bytes
      .read_from_offset::<u32>(offset)
      .wrap_error_lazy(|| format!("Failed to read resource name hash at {:#02x}", offset))
  }

  fn internal_get_flags(bytes: &[u8], ptr: usize) -> error::Result<u16> {
    let offset = ptr + mem::size_of::<u32>();

    bytes
      .read_from_offset(offset)
      .wrap_error_lazy(|| format!("Failed to read resource flags at {:#02x}", offset))
  }

  fn internal_get_name_offset(bytes: &[u8], ptr: usize) -> error::Result<u32> {
    bytes
      .read_from_offset(ptr)
      .wrap_error_lazy(|| format!("Failed to read resource name offset at {:#02x}", ptr))
  }
}
