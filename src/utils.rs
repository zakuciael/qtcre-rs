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

use std::borrow::Cow;
use std::fmt;
use std::fmt::LowerHex;
use std::path::{Path, PathBuf};

/// Prints the value in the lowercase hex with the 0x prefix
pub(crate) fn to_pretty_hex<T: LowerHex>(value: &T, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "{:#02x}", value)
}

/// Implementation based on a src code from `path-slash` crate:
/// https://github.com/rhysd/path-slash/blob/master/src/lib.rs#L84
/// Credits: rhysd (https://github.com/rhysd)
pub fn str_to_unix_path(s: &str) -> Cow<'_, Path> {
  let mut buf = String::new();

  let mut iter = s.char_indices().peekable();
  while let Some((i, c)) = iter.next() {
    if i == 0 && iter.peek().is_some_and(|(_, c)| c == &':') {
      let _ = iter.next(); // Skip ':' char
      if iter.peek().is_some_and(|(_, c)| c == &'\\') {
        let _ = iter.next(); // Skip separator char
      }

      buf.reserve(s.len() - 2);
      buf.push('/');
    } else if c == '\\' {
      if buf.is_empty() {
        buf.reserve(s.len());
        buf.push_str(&s[..i]);
      }
      buf.push('/');
    } else if !buf.is_empty() {
      buf.push(c);
    }
  }

  if buf.is_empty() {
    Cow::Borrowed(Path::new(s))
  } else {
    Cow::Owned(PathBuf::from(buf))
  }
}

pub mod __private {
  use core::fmt;
  use std::fmt::Write;

  /// A port of Qt's internal [`qt_hash()`](https://codebrowser.dev/qt6/qtbase/src/corelib/tools/qhash.cpp.html#_Z7qt_hash11QStringViewj)
  /// function
  pub fn qt_hash<K: AsRef<str>>(key: &K, chained: u32) -> u32 {
    let key = key.as_ref();
    let chars: Vec<u16> = key.encode_utf16().collect();
    let mut result = chained;

    for ch in chars {
      result = (result << 4) + ch as u32;
      result ^= (result & 0xf0000000) >> 23;
      result &= 0x0fffffff;
    }

    result
  }

  /// Implementation based on a src code from `anyhow` crate:
  /// https://github.com/dtolnay/anyhow/blob/master/src/fmt.rs
  /// Credits: David Tolnay (https://github.com/dtolnay)
  pub struct Indented<'a, D> {
    pub inner: &'a mut D,
    pub number: Option<usize>,
    pub started: bool,
  }

  impl<T> Write for Indented<'_, T>
  where
    T: Write,
  {
    fn write_str(&mut self, s: &str) -> fmt::Result {
      for (i, line) in s.split('\n').enumerate() {
        if !self.started {
          self.started = true;
          match self.number {
            Some(number) => write!(self.inner, "{: >5}: ", number)?,
            None => self.inner.write_str("    ")?,
          }
        } else if i > 0 {
          self.inner.write_char('\n')?;
          if self.number.is_some() {
            self.inner.write_str("       ")?;
          } else {
            self.inner.write_str("    ")?;
          }
        }

        self.inner.write_str(line)?;
      }

      Ok(())
    }
  }
}

/// Macro around [`qt_hash`](crate::utils::__private::qt_hash)
/// function allowing optional function arguments.
macro_rules! qt_hash {
  ($key: expr, $chained: expr) => {
    crate::utils::__private::qt_hash($key, $chained)
  };
  ($key: expr) => {
    crate::utils::__private::qt_hash($key, 0)
  };
}

macro_rules! to_hex {
  ($value: expr) => {
    format!("{:#02x}", $value)
  };
}

pub(crate) use qt_hash;
pub(crate) use to_hex;

#[cfg(test)]
mod test {
  use super::*;

  mod qt_hash {
    use super::*;

    #[test]
    fn should_generate_correct_hash() {
      assert_eq!(qt_hash!(&"certs"), 6932915);
      assert_eq!(qt_hash!(&"Client"), 77790292);
      assert_eq!(qt_hash!(&"client.p12"), 207230626);
    }
  }

  mod str_to_unix_path {
    use super::*;

    #[test]
    fn should_convert_windows_paths() {
      for path in [
        "C:\\images\\small.jpg",
        "D:\\images\\small.jpg",
        "X:\\images\\small.jpg",
      ] {
        let result = str_to_unix_path(path);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.as_ref(), Path::new("/images/small.jpg"));
      }

      let result = str_to_unix_path("\\images\\small.jpg");
      assert!(matches!(result, Cow::Owned(_)));
      assert_eq!(result.as_ref(), Path::new("/images/small.jpg"));

      let result = str_to_unix_path(".\\images\\small.jpg");
      assert!(matches!(result, Cow::Owned(_)));
      assert_eq!(result.as_ref(), Path::new("./images/small.jpg"));

      let result = str_to_unix_path("..\\images\\small.jpg");
      assert!(matches!(result, Cow::Owned(_)));
      assert_eq!(result.as_ref(), Path::new("../images/small.jpg"));

      let result = str_to_unix_path("images\\small.jpg");
      assert!(matches!(result, Cow::Owned(_)));
      assert_eq!(result.as_ref(), Path::new("images/small.jpg"));
    }

    #[test]
    fn should_leave_unix_paths_intact() {
      let result = str_to_unix_path("/images/small.jpg");
      assert!(matches!(result, Cow::Borrowed(_)));
      assert_eq!(result.as_ref(), Path::new("/images/small.jpg"));

      let result = str_to_unix_path("./images/small.jpg");
      assert!(matches!(result, Cow::Borrowed(_)));
      assert_eq!(result.as_ref(), Path::new("./images/small.jpg"));

      let result = str_to_unix_path("../images/small.jpg");
      assert!(matches!(result, Cow::Borrowed(_)));
      assert_eq!(result.as_ref(), Path::new("../images/small.jpg"));

      let result = str_to_unix_path("images/small.jpg");
      assert!(matches!(result, Cow::Borrowed(_)));
      assert_eq!(result.as_ref(), Path::new("images/small.jpg"));
    }
  }
}
