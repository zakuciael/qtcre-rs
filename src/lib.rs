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

pub use error::{Error, Result};
pub use parsers::default::ResourceReader;
pub use parsers::header::RCCFileHeaderReader;
pub use parsers::tree::ResourceTreeReader;

pub(crate) mod bytes;
pub(crate) mod constants;
mod error;
mod parsers;
pub mod types;
mod utils;

// TODO: Create unit tests to verify proper data reads and handling of "out of bounds" errors
// TODO: Better wording for error messages
// TODO: Finish documentation
// TODO: Implement ResourceTreeReader
// A reader that reads the resource tree one by one and outputs events
// used to create visual file trees

#[cfg(test)]
mod tests {
  use crate::types::Resource;
  use crate::{error, ResourceReader};

  #[test]
  fn temp() -> error::Result<()> {
    let src: [u8; 24] = [
      0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    // let mut buf = vec![0u8; 8];

    let reader = ResourceReader::from_bytes(&src, 0, 8, 0, 3)?;
    let resource = Resource::derive(0, &reader)?;

    // println!("{:?}", resource.name()?);
    reader.find("/test/123.png")?;
    Ok(())
  }
}

/*#[cfg(test)]
mod test {
  use std::fs;

  use super::*;

  #[test]
  fn temp() {
    /*let file = fs::read("./tests/fixtures/rcc/none.rcc").expect("Failed to read RCC file");
    let root = QtResourceRoot::from_rcc(&file).expect("Failed to parse RCC file");*/

    let file = fs::read("./tests/fixtures/gfclient.exe").expect("Failed to read RCC file");
    let root =
      QtResourceRoot::new(&file, 0x2f88f0, 0x2f87a0, 0x0, 2).expect("Failed to parse RCC file");

    if let Ok(Some(QtResource::Directory(res))) = root.find("/Client/images/") {
      println!("{:?}", res);
      println!("{:?}", res.children());
    }
  }
}*/
