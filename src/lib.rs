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

pub(crate) mod bytes;
pub(crate) mod constants;
pub mod error;
pub mod readers;
pub mod types;
mod utils;

// TODO: Better wording for error messages
// TODO: Finish documentation
// TODO: Implement ResourceTreeReader
// A reader that reads the resource tree one by one and outputs events
// used to create visual file trees

#[cfg(test)]
mod tests {
  use crate::{error, readers::ResourceReader};
  use std::fs;

  #[test]
  fn temp() -> error::Result<()> {
    let file = fs::read("./tests/fixtures/rcc/none.rcc").expect("Failed to read RCC file");

    /*let file = fs::read("./tests/fixtures/gfclient.exe").expect("Failed to read RCC file");
    let root =
      QtResourceRoot::new(&file, 0x2f88f0, 0x2f87a0, 0x0, 2).expect("Failed to parse RCC file");*/

    let reader = ResourceReader::from_rcc(&file)?;
    println!("{:?}", reader.find("/images/small.jpg")?);
    Ok(())
  }
}
