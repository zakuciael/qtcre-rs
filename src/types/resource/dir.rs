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

use byteorder::{BigEndian, ReadBytesExt};
use educe::Educe;

use crate::bytes::ReadFromOffset;
use crate::error;
use crate::error::WrapError;
use crate::parsers::default::ResourceReader;
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
  pub(crate) root: &'a ResourceReader<'a>,
}

impl<'a> ResourceBase for ResourceDirectory<'a> {}

impl<'a> ResourceDirectory<'a> {
  pub(crate) fn new(index: u32, root: &'a ResourceReader<'a>) -> ResourceDirectory<'a> {
    Self {
      ptr: root.find_ptr(index),
      absolute_path: PathBuf::new(),
      root,
    }
  }

  pub fn name(&self) -> error::Result<String> {
    Self::internal_get_name(self.root.bytes, self.ptr, self.root.name_offset)
  }

  pub fn children(&self) -> error::Result<Vec<Resource<'a>>> {
    let child_count = self.child_count()?;
    let child_offset = self.child_offset()?;

    let mut childs = vec![];
    for child in 0..child_count {
      let mut node = Resource::derive(child_offset + child, self.root)?;
      node.set_absolute_path(self.absolute_path.join(node.name()?));
      childs.push(node);
    }

    Ok(childs)
  }

  pub(crate) fn hash(&self) -> error::Result<u32> {
    Self::internal_get_hash(self.root.bytes, self.ptr, self.root.name_offset)
  }

  pub(crate) fn child_count(&self) -> error::Result<u32> {
    self
      .root
      .bytes
      .read_from_offset(self.ptr + mem::size_of::<u32>() + mem::size_of::<u16>())
      .wrap_error_lazy(|| {
        format!(
          "Failed to read resource child count at {}",
          to_hex!(self.ptr)
        )
      })
  }

  pub(crate) fn child_offset(&self) -> error::Result<u32> {
    self
      .root
      .bytes
      .read_from_offset(self.ptr + mem::size_of::<u32>() * 2 + mem::size_of::<u16>())
      .wrap_error_lazy(|| {
        format!(
          "Failed to read resource child offset at {}",
          to_hex!(self.ptr)
        )
      })
  }
}
