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

use std::mem;
use std::path::PathBuf;

use educe::Educe;

use crate::bytes::ReadFromOffset;
use crate::error;
use crate::error::WrapError;
use crate::readers::ResourceReader;
use crate::types::resource::base::ResourceBase;
use crate::types::Resource;
use crate::utils::{to_hex, to_pretty_hex};

#[derive(Educe)]
#[educe(Debug)]
pub struct ResourceDirectory<'a> {
  #[educe(Debug(method = "to_pretty_hex"))]
  pub(crate) ptr: usize,
  pub(crate) absolute_path: PathBuf,
  #[educe(Debug(ignore))]
  pub(crate) reader: &'a ResourceReader<'a>,
}

impl<'a> ResourceBase for ResourceDirectory<'a> {}

impl<'a> ResourceDirectory<'a> {
  pub(crate) fn new(index: u32, reader: &'a ResourceReader<'a>) -> ResourceDirectory<'a> {
    Self {
      ptr: reader.find_ptr(index),
      absolute_path: PathBuf::new(),
      reader,
    }
  }

  pub fn name(&self) -> error::Result<String> {
    Self::internal_get_name(self.reader.bytes, self.ptr, self.reader.name_offset)
  }

  pub fn children(&self) -> error::Result<Vec<Resource<'a>>> {
    let child_count = self.child_count()?;
    let child_offset = self.child_offset()?;

    let mut childs = vec![];
    for child in 0..child_count {
      let mut node = Resource::derive(child_offset + child, self.reader)?;
      node.set_absolute_path(self.absolute_path.join(node.name()?));
      childs.push(node);
    }

    Ok(childs)
  }

  pub(crate) fn hash(&self) -> error::Result<u32> {
    Self::internal_get_hash(self.reader.bytes, self.ptr, self.reader.name_offset)
  }

  pub(crate) fn child_count(&self) -> error::Result<u32> {
    let offset = self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>();

    self
      .reader
      .bytes
      .read_from_offset(offset)
      .wrap_error_lazy(|| format!("Failed to read resource child count at {:#02x}", offset))
  }

  pub(crate) fn child_offset(&self) -> error::Result<u32> {
    let offset = self.ptr + mem::size_of::<u32>() * 2 + mem::size_of::<u16>();

    self
      .reader
      .bytes
      .read_from_offset(offset)
      .wrap_error_lazy(|| format!("Failed to read resource child offset at {:#02x}", offset))
  }
}

#[cfg(test)]
mod tests {
  use crate::readers::ResourceReader;
  use crate::types::ResourceDirectory;

  #[test]
  fn should_correctly_read_resource() {
    let bytes: &[u8] = &[
      0x00, 0x00, 0x00, 0x00, // Name offset
      0x00, 0x02, // Flags
      0x00, 0x00, 0x00, 0x07, // Child count
      0x00, 0x00, 0x00, 0xAF, // Child offset,
      0xFF, 0xFF, // Spacing
      0x00, 0x06, // Name length
      0x07, 0x03, 0x7D, 0xC3, // Name hash
      0x00, 0x69, 0x00, 0x6D, 0x00, 0x61, 0x00, 0x67, 0x00, 0x65, 0x00,
      0x73, // "images" string
      0xFF, 0xFF, // Spacing
      0x00, 0x00, // Data
    ];

    let reader = ResourceReader::from_bytes(&bytes, 0, 16, 36, 3).expect("Failed to create reader");
    let resource = ResourceDirectory::new(0, &reader);

    let name = resource.name();
    let child_count = resource.child_count();
    let child_offset = resource.child_offset();

    assert!(name.is_ok());
    assert_eq!(name.unwrap(), "images");

    assert!(child_count.is_ok());
    assert_eq!(child_count.unwrap(), 0x07);

    assert!(child_offset.is_ok());
    assert_eq!(child_offset.unwrap(), 0xAF);
  }
}
