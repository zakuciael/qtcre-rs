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

/// Represents Qt's internal flags
#[repr(u16)]
pub enum ResourceFlags {
  /// Resource is compressed using the [zlib](https://zlib.net/) library
  ZlibCompression = 0x01,
  /// Resource is a directory
  Directory = 0x02,
  /// Resource is compressed using the [zstd](http://facebook.github.io/zstd/) library
  ZstdCompression = 0x04,
}
