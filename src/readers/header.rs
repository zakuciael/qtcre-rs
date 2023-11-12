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

use std::io::{Cursor, Read};

use crate::bytes::ReadFromOffset;
use crate::constants::RCC_FILE_HEADER_MAGIC;
use byteorder::{BigEndian, ReadBytesExt};

use crate::error;
use crate::error::{Error, WrapError};

#[derive(Debug)]
pub struct RCCFileHeaderReader {
  pub magic: [u8; 4],
  pub format_version: u32,
  pub struct_offset: usize,
  pub data_offset: usize,
  pub name_offset: usize,
  pub overall_flags: Option<u32>,
}

impl RCCFileHeaderReader {
  pub fn new<T: AsRef<[u8]>>(bytes: &T) -> error::Result<Self> {
    let mut reader = Cursor::new(bytes.as_ref());

    let magic = {
      let mut buf = [0u8; 4];
      reader
        .read_exact(&mut buf)
        .wrap_error("Failed to read magic bytes")?;

      buf
    };

    if &magic != RCC_FILE_HEADER_MAGIC {
      return Err(Error::InvalidHeaderMagic {
        received: magic,
        expected: *RCC_FILE_HEADER_MAGIC,
      });
    }

    let format_version = reader
      .read_u32::<BigEndian>()
      .wrap_error("Failed to read format version")?;

    let struct_offset = reader
      .read_u32::<BigEndian>()
      .wrap_error("Failed to read struct offset")? as usize;
    let data_offset = reader
      .read_u32::<BigEndian>()
      .wrap_error("Failed to read data offset")? as usize;
    let name_offset = reader
      .read_u32::<BigEndian>()
      .wrap_error("Failed to read name offset")? as usize;

    let overall_flags = if format_version >= 3 {
      Some(
        reader
          .read_u32::<BigEndian>()
          .wrap_error("Failed to read overall flags")?,
      )
    } else {
      None
    };

    Ok(Self {
      magic,
      format_version,
      struct_offset,
      data_offset,
      name_offset,
      overall_flags,
    })
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn should_error_on_invalid_magic_bytes() {
    assert!(matches!(
      RCCFileHeaderReader::new(&[0x71, 0x00, 0x65, 0x05, 0x00, 0x01, 0x02, 0x03]),
      Err(Error::InvalidHeaderMagic {
        received: [0x71, 0x00, 0x65, 0x05],
        expected: [0x71, 0x72, 0x65, 0x73]
      })
    ));
  }

  #[test]
  fn should_error_when_buffer_is_too_small() {
    assert!(matches!(
      RCCFileHeaderReader::new(&[0u8; 3]),
      Err(Error::IO(_))
    ));
    assert!(matches!(
      RCCFileHeaderReader::new(&[0x71, 0x72, 0x65, 0x73, 0x00]),
      Err(Error::IO(_))
    ));
    assert!(matches!(
      RCCFileHeaderReader::new(&[0x71, 0x72, 0x65, 0x73, 0x00, 0x01, 0x02, 0x03, 0x04]),
      Err(Error::IO(_))
    ));
  }

  #[test]
  fn should_parse_file_header() {
    assert!(matches!(
      RCCFileHeaderReader::new(&[
        0x71, 0x72, 0x65, 0x73, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0xF6, 0x82, 0x00, 0x00, 0x00,
        0x18, 0x00, 0x00, 0xF6, 0x58, 0x00, 0x00, 0x00, 0x00
      ]),
      Ok(RCCFileHeaderReader {
        magic: [0x71, 0x72, 0x65, 0x73],
        format_version: 3,
        struct_offset: 0xF682,
        data_offset: 0x18,
        name_offset: 0xF658,
        overall_flags: Some(0x00),
      })
    ));
  }
}
