/*
 * qtcre-rs
 * Copyright (c) 2023 Krzysztof Saczuk <me@krzysztofsaczuk.pl>.
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

use crate::types::ResourceFlags;

/// Represents a compression algorithm used by Qt's RCC tool to compress the payload
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CompressionAlgorithm {
  /// Contents are not compressed
  None,
  /// Contents are compressed using the [zlib](https://zlib.net/) library
  Zlib,
  /// Contents are compressed using the [zstd](http://facebook.github.io/zstd/) library
  Zstd,
}

impl From<u16> for CompressionAlgorithm {
  fn from(value: u16) -> Self {
    let compression_flags =
      value & (ResourceFlags::ZlibCompression as u16 | ResourceFlags::ZstdCompression as u16);

    match compression_flags {
      _ if compression_flags == ResourceFlags::ZlibCompression as u16 => CompressionAlgorithm::Zlib,
      _ if compression_flags == ResourceFlags::ZstdCompression as u16 => CompressionAlgorithm::Zstd,
      _ => CompressionAlgorithm::None,
    }
  }
}
