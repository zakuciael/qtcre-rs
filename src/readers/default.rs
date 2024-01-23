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

use std::cmp::Ordering;
use std::path::Component::Normal;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use educe::Educe;
use path_absolutize::Absolutize;

use crate::constants::SUPPORTED_FORMAT_VERSION;
use crate::error;
use crate::error::Error;
use crate::readers::RCCFileHeaderReader;
use crate::types::Resource;
use crate::utils::{qt_hash, str_to_unix_path, to_pretty_hex};

#[derive(Educe)]
#[educe(Debug)]
pub struct ResourceReader<'a> {
  #[educe(Debug(method = "to_pretty_hex"))]
  pub(crate) struct_offset: usize,
  #[educe(Debug(method = "to_pretty_hex"))]
  pub(crate) name_offset: usize,
  #[educe(Debug(method = "to_pretty_hex"))]
  pub(crate) data_offset: usize,
  pub(crate) format_version: u32,

  #[educe(Debug(ignore))]
  pub(crate) bytes: &'a [u8],
}

impl<'a> ResourceReader<'a> {
  pub fn from_bytes<T: AsRef<[u8]>>(
    bytes: &'a T,
    struct_offset: usize,
    name_offset: usize,
    data_offset: usize,
    format_version: u32,
  ) -> error::Result<ResourceReader<'a>> {
    let bytes: &'a [u8] = bytes.as_ref();
    let len = bytes.len();

    if struct_offset >= len {
      return Err(Error::InvalidOffset {
        name: "struct_offset",
        received: struct_offset,
        expected: len,
      });
    }

    if name_offset >= len {
      return Err(Error::InvalidOffset {
        name: "name_offset",
        received: name_offset,
        expected: len,
      });
    }

    if data_offset >= len {
      return Err(Error::InvalidOffset {
        name: "data_offset",
        received: data_offset,
        expected: len,
      });
    }

    if format_version > 3 {
      return Err(Error::UnsupportedVersion {
        received: format_version,
        expected: SUPPORTED_FORMAT_VERSION,
      });
    }

    Ok(Self {
      format_version,
      struct_offset,
      name_offset,
      data_offset,
      bytes,
    })
  }

  pub fn find<T: AsRef<str>>(&self, path: T) -> error::Result<Option<Resource>> {
    let path = str_to_unix_path(path.as_ref());
    let path = path.absolutize_from("/").unwrap(); // This function never returns an errors

    let mut resource_path = PathBuf::from("/");
    let root = {
      let mut res = Resource::derive(0, self)?;
      res.set_absolute_path(&resource_path);

      match res {
        Resource::File(_) => {
          return Err(Error::InvalidData(anyhow!(
            "An invalid file was detected, first resource should always be a directory"
          )));
        }
        Resource::Directory(resource) => resource,
      }
    };

    if path.eq(Path::new("/")) {
      return Ok(Some(Resource::Directory(root)));
    }

    let mut child_count = root.child_count()?;
    let mut child_offset = root.child_offset()?;
    let mut segments = path
      .components()
      .filter_map(|component| {
        if let Normal(segment) = component {
          Some(segment.to_string_lossy().to_string())
        } else {
          None
        }
      })
      .peekable();

    while let Some(segment) = segments.next() {
      let Some(mut node) = self.binary_search(&segment, child_count, child_offset)? else {
        break;
      };

      resource_path = resource_path.join(node.name()?);
      node.set_absolute_path(&resource_path);
      return Ok(match node {
        Resource::Directory(node) => {
          if segments.peek().is_some() {
            child_count = node.child_count()?;
            child_offset = node.child_offset()?;
            continue;
          }

          Some(Resource::Directory(node))
        }
        Resource::File(node) => {
          if segments.peek().is_some() {
            return Ok(None);
          }

          Some(Resource::File(node))
        }
      });
    }

    Ok(None)
  }

  pub fn from_rcc<T: AsRef<[u8]>>(bytes: &'a T) -> error::Result<ResourceReader<'a>> {
    let reader = RCCFileHeaderReader::new(bytes)?;

    Self::from_bytes(
      bytes,
      reader.struct_offset,
      reader.name_offset,
      reader.data_offset,
      reader.format_version,
    )
  }

  pub(crate) fn find_ptr(&self, index: u32) -> usize {
    let offset = index * (14 + (if self.format_version >= 2 { 8 } else { 0 }));

    self.struct_offset + offset as usize
  }

  fn binary_search(
    &self,
    key: &str,
    child_count: u32,
    child_offset: u32,
  ) -> error::Result<Option<Resource>> {
    let mut left = 0;
    let mut right = child_count;

    while left < right {
      let mid = (left + right) / 2;
      let node = Resource::derive(child_offset + mid, self)?;

      match node.hash()?.cmp(&qt_hash!(&key)) {
        Ordering::Equal => return Ok(Some(node)),
        Ordering::Less => left = mid + 1,
        Ordering::Greater => right = mid,
      }
    }

    Ok(None)
  }
}
