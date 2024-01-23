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

use std::mem;
use std::path::Path;

use byteorder::{BigEndian, ReadBytesExt};

use crate::bytes::ReadFromOffset;
pub use dir::ResourceDirectory;
pub use file::ResourceFile;

use crate::error;
use crate::error::WrapError;
use crate::readers::ResourceReader;
use crate::types::ResourceFlags;
use crate::utils::to_hex;

mod base;
mod dir;
mod file;

#[derive(Debug)]
pub enum Resource<'a> {
  File(ResourceFile<'a>),
  Directory(ResourceDirectory<'a>),
}

impl<'a> Resource<'a> {
  pub(crate) fn derive(index: u32, reader: &'a ResourceReader<'a>) -> error::Result<Resource<'a>> {
    let offset = reader.find_ptr(index) + mem::size_of::<u32>();
    let flags = reader
      .bytes
      .read_from_offset::<u16>(offset)
      .wrap_error_lazy(|| format!("Failed to read resource flags at {:#02x}", offset))?;

    Ok(if flags & ResourceFlags::Directory as u16 > 0 {
      Resource::Directory(ResourceDirectory::new(index, reader))
    } else {
      Resource::File(ResourceFile::new(index, reader))
    })
  }

  pub fn is_file(&self) -> bool {
    matches!(self, Resource::File(_))
  }

  pub fn is_dir(&self) -> bool {
    matches!(self, Resource::Directory(_))
  }

  pub fn name(&self) -> error::Result<String> {
    match &self {
      Resource::File(res) => res.name(),
      Resource::Directory(res) => res.name(),
    }
  }

  pub fn hash(&self) -> error::Result<u32> {
    match &self {
      Resource::File(res) => res.hash(),
      Resource::Directory(res) => res.hash(),
    }
  }

  pub(crate) fn set_absolute_path<T: AsRef<Path>>(&mut self, path: T) {
    match self {
      Resource::File(res) => res.absolute_path = path.as_ref().to_path_buf(),
      Resource::Directory(res) => res.absolute_path = path.as_ref().to_path_buf(),
    }
  }
}
