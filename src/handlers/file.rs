// Copyright 2017 Dmytro Milinevskyi <dmilinevskyi@gmail.com>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate parking_lot;
use self::parking_lot::Mutex;

use std::fs::{File, OpenOptions, create_dir_all};
use std::path::Path;
use std::io;
use std::io::Write;

use handlers::Handler;

/// The errors that might occur during creation of the handler.
#[derive(Debug)]
pub enum FileHandlerError {
    /// Any kind of I/O Error.
    IoError(io::Error),
}

impl From<io::Error> for FileHandlerError {
    fn from(e: io::Error) -> Self {
        FileHandlerError::IoError(e)
    }
}

struct Context {
    file: File,
}

impl Context {
    fn new(path: &Path) -> Result<Self, FileHandlerError> {
        if let Some(dir) = path.parent() {
            create_dir_all(dir)?;
        }
        Ok(Context {
            file: OpenOptions::new().append(true).create(true).open(path)?,
        })
    }

    fn emit(&mut self, msg: &[u8]) {
        let _ = self.file.write_all(msg);
    }
}

/// Pushes formatted log record into a file.
///
/// The directories to the log file are created automatically.
pub fn handler(path: &Path) -> Result<Handler, FileHandlerError> {
    let ctx = Context::new(path)?;
    let ctx = Mutex::new(ctx);
    Ok(Box::new(move |record| {
        let mut ctx = ctx.lock();
        ctx.emit(record.formatted().as_bytes());
    }))
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use self::tempdir::TempDir;

    use super::*;

    #[test]
    fn test_file() {
        let dir = TempDir::new("wp-f").unwrap();
        let path = dir.path().join("logs").join("test.log");
        let mut ctx = Context::new(&path).unwrap();

        assert!(path.exists());
        assert_eq!(path.metadata().unwrap().len(), 0);
        ctx.emit("x".as_bytes());
        assert_eq!(path.metadata().unwrap().len(), 1);
        ctx.emit("x".as_bytes());
        assert_eq!(path.metadata().unwrap().len(), 2);
    }

    #[test]
    fn test_file_invalid() {
        let dir = TempDir::new("wp-f").unwrap();
        let ctx = Context::new(&dir.path());
        let err = ctx.err().unwrap();

        assert!(format!("{:?}", err).contains("directory"));
    }
}
