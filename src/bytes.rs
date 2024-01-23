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

use std::io::{ErrorKind, Seek, SeekFrom};
use std::{io, mem};

macro_rules! from_be_bytes_ext_imp {
  ($type:ty) => {
    impl FromBeBytesExt for $type {
      fn from_be_bytes_ext(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
      }
    }
  };
}

pub(crate) trait FromBeBytesExt {
  fn from_be_bytes_ext(bytes: &[u8]) -> Self;
}

pub(crate) trait ReadFromOffset {
  fn read_from_offset<T: FromBeBytesExt>(&self, offset: usize) -> io::Result<T>;
}

from_be_bytes_ext_imp!(u8);
from_be_bytes_ext_imp!(u16);
from_be_bytes_ext_imp!(u32);
from_be_bytes_ext_imp!(u64);

from_be_bytes_ext_imp!(i8);
from_be_bytes_ext_imp!(i16);
from_be_bytes_ext_imp!(i32);
from_be_bytes_ext_imp!(i64);

impl ReadFromOffset for [u8] {
  fn read_from_offset<T: FromBeBytesExt>(&self, offset: usize) -> io::Result<T> {
    let buf = self
      .get(offset..offset + mem::size_of::<T>())
      .ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    Ok(T::from_be_bytes_ext(buf))
  }
}
