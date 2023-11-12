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

//! Contains crate's error types

use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Write};
use std::io;
use std::io::ErrorKind;
use std::string::FromUtf16Error;

use anyhow::{anyhow, Chain};

use crate::utils::__private::Indented;

/// Specialized [`Result`] type for crate's errors
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors while parsing Qt's resources.
#[derive(thiserror::Error)]
pub enum Error {
  /// Provided data offset is out of bounds.
  #[error("The offset \"{name}\" is out of bounds, expected: < {expected}, received: {received}.")]
  InvalidOffset {
    name: &'static str,
    received: usize,
    expected: usize,
  },

  /// Provided format version is unsupported.
  #[error(
    "The specified format version is not supported, expected: <= {expected}, received: {received}."
  )]
  UnsupportedVersion { received: u32, expected: u32 },

  /// Expected RCC header magic does not match.
  #[error("The header magic bytes are not valid, expected: {expected:?}, received: {received:?}")]
  InvalidHeaderMagic {
    received: [u8; 4],
    expected: [u8; 4],
  },

  /// Out of bounds.
  ///
  /// Catch-all for bounds check errors.
  #[error("Address is out of bounds")]
  OutOfBounds(#[source] anyhow::Error),

  /// I/O error.
  ///
  /// Catch-all for I/O related errors.
  #[error("Unexpected I/O error occurred")]
  IO(#[source] anyhow::Error),

  /// Invalid data.
  ///
  /// Structured data was found which is invalid or corrupted.
  #[error("Data is invalid or corrupted")]
  InvalidData(#[source] anyhow::Error),
}

/// Implements methods to wrap [`std`] errors with our custom [`Error`] type
/// while also provide additional context
pub(crate) trait WrapError<T> {
  /// Wrap the error value with additional context.
  fn wrap_error<C>(self, context: C) -> Result<T>
  where
    C: Display + Send + Sync + 'static;

  /// Wrap the error value with additional context that is evaluated lazily
  /// only once an error does occur.
  fn wrap_error_lazy<C, F>(self, f: F) -> Result<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C;
}

/// Implementation based on `anyhow` crate src:
/// https://github.com/dtolnay/anyhow/blob/master/src/fmt.rs
impl Debug for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    if f.alternate() {
      return Debug::fmt(self, f);
    }

    if let Some(cause) = self.source() {
      // We have a cause, let's swap it with this error
      write!(f, "{}", cause)?;
      write!(f, "\n\nCaused by:")?;

      let multiple = cause.source().is_some();
      let mut print_error = |err: &dyn StdError, n| -> std::fmt::Result {
        writeln!(f)?;
        let mut indented = Indented {
          inner: f,
          number: if multiple { Some(n) } else { None },
          started: false,
        };
        write!(indented, "{}", err)
      };

      print_error(&self, 0)?;
      if let Some(cause) = cause.source() {
        for (n, error) in Chain::new(cause).enumerate() {
          print_error(error, n)?;
        }
      }
    } else {
      // No cause? Just print this error
      write!(f, "{}", self)?;
    }

    Ok(())
  }
}

impl From<io::Error> for Error {
  fn from(value: io::Error) -> Self {
    Self::IO(anyhow!(value))
  }
}

impl<T> WrapError<T> for std::result::Result<T, io::Error> {
  fn wrap_error<C>(self, context: C) -> Result<T>
  where
    C: Display + Send + Sync + 'static,
  {
    match self {
      Ok(val) => Ok(val),
      Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
        Err(Error::OutOfBounds(anyhow!(context.to_string())))
      }
      Err(err) => Err(Error::IO(anyhow!(err).context(context))),
    }
  }

  fn wrap_error_lazy<C, F>(self, context: F) -> Result<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C,
  {
    match self {
      Ok(val) => Ok(val),
      Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
        Err(Error::OutOfBounds(anyhow!(context().to_string())))
      }
      Err(err) => Err(Error::IO(anyhow!(err).context(context()))),
    }
  }
}

impl<T> WrapError<T> for std::result::Result<T, FromUtf16Error> {
  fn wrap_error<C>(self, context: C) -> Result<T>
  where
    C: Display + Send + Sync + 'static,
  {
    match self {
      Ok(val) => Ok(val),
      Err(err) => Err(Error::InvalidData(anyhow!(err).context(context))),
    }
  }

  fn wrap_error_lazy<C, F>(self, context: F) -> Result<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C,
  {
    match self {
      Ok(val) => Ok(val),
      Err(err) => Err(Error::InvalidData(anyhow!(err).context(context()))),
    }
  }
}
