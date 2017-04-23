// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs::{File, OpenOptions, create_dir_all, rename};
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use std::io::Write;

use handlers::Handler;

struct Context {
    path: PathBuf,
    logs: Vec<PathBuf>,
    size: u64,
    current: u64,
    file: File,
}

impl Context {
    fn open(path: &Path) -> File {
        OpenOptions::new().append(true).create(true).open(path).unwrap()
    }

    fn logs(path: &Path, count: usize) -> Vec<PathBuf> {
        (0..count-1).rev().map(|r| PathBuf::from(format!("{}.{}", path.display(), r))).collect()
    }

    fn new(path: &Path, count: usize, size: u64) -> Self {
        match path.parent() {
            Some(dir) => create_dir_all(dir).unwrap(),
            None => {},
        };
        let file = Self::open(path);
        assert!(size > 0);
        assert!(count > 0);
        Context {
            path: path.into(),
            logs: Self::logs(path, count),
            size: size,
            current: file.metadata().unwrap().len(),
            file: file,
        }
    }

    fn emit(&mut self, msg: &[u8]) {
        let _ = self.file.write(msg);
        self.current += msg.len() as u64;
        if self.current >= self.size {
            let rlen = self.logs.len();
            for i in 1..rlen {
                let old = &self.logs[i];
                if old.exists() {
                    let _ = rename(old, &self.logs[i-1]);
                }
            }
            let _ = self.file.flush();
            let _ = rename(&self.path, &self.logs[rlen-1]);
            self.file = Self::open(&self.path);
            self.current = 0;
        }
    }
}

pub fn handler(path: &Path, count: usize, size: u64) -> Handler<'static> {
    let ctx = Mutex::new(Context::new(path, count, size));
    Box::new(move |record| {
        let mut ctx = ctx.lock().unwrap();
        ctx.emit(record.formatted().as_bytes());
    })
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
        let mut ctx = super::Context::new(&path, count, size);
        let mut logs = super::Context::logs(&path, count+1);
        logs.reverse();
        let elogs = &logs[..logs.len()-1];
        let flog = &logs[logs.len()-1];

        assert!(path.exists());
        assert_eq!(path.metadata().unwrap().len(), 0);

        push(&mut ctx, size/2);
        assert_eq!(path.metadata().unwrap().len(), size/2);
        tlogs(elogs, size, 0);

        push(&mut ctx, size/2);
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
