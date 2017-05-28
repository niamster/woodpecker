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

use std::fs::{File, OpenOptions, create_dir_all, rename};
use std::path::{Path, PathBuf};
use std::fmt;
use std::io;
use std::io::Write;

use handlers::Handler;

/// The errors that might occur during creation of the handler.
pub enum RotatingFileHandlerError {
    /// Any kind of I/O Error.
    IoError(io::Error),
    /// Log file size is invalid.
    SizeError(u64),
    /// Log file count is invalid.
    CountError(usize),
}

impl From<io::Error> for RotatingFileHandlerError {
    fn from(e: io::Error) -> Self {
        RotatingFileHandlerError::IoError(e)
    }
}

impl fmt::Debug for RotatingFileHandlerError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RotatingFileHandlerError::IoError(ref err) => write!(f, "{}", err),
            RotatingFileHandlerError::SizeError(ref size) => write!(f, "Invalid size {}", size),
            RotatingFileHandlerError::CountError(ref count) => write!(f, "Invalid count {}", count),
        }
    }
}

struct Context {
    path: PathBuf,
    logs: Vec<PathBuf>,
    size: u64,
    current: u64,
    file: File,
}

impl Context {
    fn open(path: &Path) -> Result<File, RotatingFileHandlerError> {
        match OpenOptions::new().append(true).create(true).open(path) {
            Ok(file) => Ok(file),
            Err(err) => Err(err.into()),
        }
    }

    fn logs(path: &Path, count: usize) -> Vec<PathBuf> {
        (0..count-1).rev().map(|r| PathBuf::from(format!("{}.{}", path.display(), r))).collect()
    }

    fn new(path: &Path, count: usize, size: u64) -> Result<Self, RotatingFileHandlerError> {
        if let Some(dir) = path.parent() {
            create_dir_all(dir)?;
        }
        let file = Self::open(path)?;
        if size == 0 {
            return Err(RotatingFileHandlerError::SizeError(size));
        }
        if count == 0 {
            return Err(RotatingFileHandlerError::CountError(count));
        }
        Ok(Context {
            path: path.into(),
            logs: Self::logs(path, count),
            size: size,
            current: file.metadata()?.len(),
            file: file,
        })
    }

    fn ____emit(&mut self, msg: &[u8]) {
        let _ = self.file.write_all(msg);
        self.current += msg.len() as u64;
    }

    fn __emit(&mut self, msg: String) {
        self.____emit(msg.as_bytes())
    }

    fn rotate(&mut self) {
        let rlen = self.logs.len();

        for i in 1..rlen {
            let res = {
                let old = &self.logs[i];
                if old.exists() {
                    rename(old, &self.logs[i - 1])
                } else {
                    Ok(())
                }
            };

            if res.is_err() {
                let msg = {
                    let old = self.logs[i].display();
                    let new = self.logs[i - 1].display();
                    format!("Failed to rename {} into {}: {}", old, new, res.unwrap_err())
                };
                self.__emit(msg);
            }
        }

        let _ = self.file.flush();

        if let Err(err) = rename(&self.path, &self.logs[rlen - 1]) {
            let msg = {
                let old = self.path.display();
                let new = self.logs[rlen - 1].display();
                format!("Failed to rename {} into {}: {}", old, new, err)
            };
            self.__emit(msg);
        }
    }

    fn emit(&mut self, msg: &[u8]) {
        self.____emit(msg);
        if self.current >= self.size {
            self.rotate();

            match Self::open(&self.path) {
                Ok(file) => {
                    self.file = file;
                    self.current = 0;
                },
                Err(err) => {
                    let msg = format!("Failed to open {}: {:?}", self.path.display(), err);
                    self.__emit(msg);
                }
            }
        }
    }
}

/// Pushes log record into a file.
///
/// The directories to the log file are created automatically.
///
/// Rotates the log file is it exceed the given `size` (in bytes).
///
/// Maintains up to `count` log files.
///
/// Each log file after rotation has a numeric suffix.
pub fn handler(path: &Path, count: usize, size: u64) -> Result<Handler, RotatingFileHandlerError> {
    let ctx = Context::new(path, count, size)?;
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

    fn push(ctx: &mut super::Context, size: u64) {
        ctx.emit("x".repeat(size as usize).as_bytes());
    }

    fn tlogs(logs: &[PathBuf], size: u64, filled: usize) {
        for i in 0..filled {
            let log = &logs[i];
            assert!(log.exists());
            assert_eq!(log.metadata().unwrap().len(), size);
        }
        for i in filled..logs.len() {
            assert!(!logs[i].exists());
        }
    }

    #[test]
    fn test_rotating_file() {
        let dir = TempDir::new("wp-rf").unwrap();
        let path = dir.path().join("logs").join("test.log");
        let count = 5;
        let size = 20;
        let mut ctx = super::Context::new(&path, count, size).unwrap();
        let mut logs = super::Context::logs(&path, count + 1);
        logs.reverse();
        let elogs = &logs[..logs.len() - 1];
        let flog = &logs[logs.len() - 1];

        assert!(path.exists());
        assert_eq!(path.metadata().unwrap().len(), 0);

        push(&mut ctx, size / 2);
        assert_eq!(path.metadata().unwrap().len(), size / 2);
        tlogs(elogs, size, 0);

        push(&mut ctx, size / 2);
        assert_eq!(path.metadata().unwrap().len(), 0);
        tlogs(elogs, size, 1);

        for i in 2..count {
            push(&mut ctx, size);
            assert_eq!(path.metadata().unwrap().len(), 0);
            tlogs(elogs, size, i);
        }

        assert!(!flog.exists());
    }
}
