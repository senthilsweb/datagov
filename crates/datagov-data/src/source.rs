//! source.rs — `Source`: the input a `DatasetReader` reads from.
//!
//! 1. Defines `Source`, wrapping either a real file path or a boxed
//!    `Read` (stdin), plus the resolved `Format` — the shared shape
//!    `inspect` (and later `profile`) hand to a `DatasetReader`.
//! 2. `open_read` yields a `Read` handle exactly once: for a path it
//!    opens the file fresh each call; for stdin it hands back the boxed
//!    reader the first time and errors on any further call (stdin can
//!    only be consumed once). Interior mutability (`RefCell`) is used so
//!    `DatasetReader::inspect` can keep its `&self, source: &Source`
//!    signature while still consuming the stdin reader.
//! 3. `uri()` reports what `Input::uri` should be set to: the path as
//!    given, or the literal `"-"` for stdin.
//! 4. `file_size_bytes()` reports the real file size for path sources;
//!    stdin has no meaningful file size and reports `0`.

use datagov_core::DatagovError;
use std::cell::RefCell;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::format::Format;

enum Origin {
    Path(PathBuf),
    Stdin(RefCell<Option<Box<dyn Read>>>),
}

/// The input a `DatasetReader` operates on: a resolved `Format` plus
/// either a file path or a one-shot boxed `Read` (stdin).
pub struct Source {
    format: Format,
    origin: Origin,
}

impl Source {
    /// A real file on disk.
    pub fn from_path(path: PathBuf, format: Format) -> Self {
        Source {
            format,
            origin: Origin::Path(path),
        }
    }

    /// Standard input, reported as `uri() == "-"`.
    pub fn from_stdin(reader: Box<dyn Read>, format: Format) -> Self {
        Source {
            format,
            origin: Origin::Stdin(RefCell::new(Some(reader))),
        }
    }

    pub fn format(&self) -> Format {
        self.format
    }

    /// The real filesystem path, when this source is a path (not stdin).
    pub fn path(&self) -> Option<&Path> {
        match &self.origin {
            Origin::Path(p) => Some(p.as_path()),
            Origin::Stdin(_) => None,
        }
    }

    /// What `Input::uri` should be set to for this source.
    pub fn uri(&self) -> String {
        match &self.origin {
            Origin::Path(p) => p.display().to_string(),
            Origin::Stdin(_) => "-".to_string(),
        }
    }

    /// The file size in bytes, or `0` for stdin (no meaningful size).
    pub fn file_size_bytes(&self) -> Result<u64, DatagovError> {
        match &self.origin {
            Origin::Path(p) => std::fs::metadata(p)
                .map(|m| m.len())
                .map_err(|source| input_not_found(p, source)),
            Origin::Stdin(_) => Ok(0),
        }
    }

    /// Open a fresh `Read` handle. For a path source this opens the file
    /// (mapping a missing file to `DatagovError::InputNotFound`). For
    /// stdin, this succeeds exactly once — a second call is an internal
    /// error, since stdin cannot be rewound or read twice.
    pub fn open_read(&self) -> Result<Box<dyn Read>, DatagovError> {
        match &self.origin {
            Origin::Path(p) => {
                let file = std::fs::File::open(p).map_err(|source| input_not_found(p, source))?;
                Ok(Box::new(file))
            }
            Origin::Stdin(cell) => cell
                .borrow_mut()
                .take()
                .ok_or_else(|| DatagovError::internal("stdin has already been consumed")),
        }
    }
}

fn input_not_found(path: &Path, source: std::io::Error) -> DatagovError {
    DatagovError::input_not_found(
        format!("input not found: {} ({source})", path.display()),
        format!("check that {} exists and is readable", path.display()),
    )
}
