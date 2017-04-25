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

use std::fs::{File, OpenOptions, create_dir_all};
use std::sync::Mutex;
use std::path::Path;
use std::io::Write;

use handlers::Handler;

struct Context {
    file: File,
}

impl Context {
    fn new(path: &Path) -> Self {
        match path.parent() {
            Some(dir) => create_dir_all(dir).unwrap(),
            None => {},
        };
        Context {
            file: OpenOptions::new().append(true).create(true).open(path).unwrap(),
        }
    }

    fn emit(&mut self, msg: &[u8]) {
        let _ = self.file.write(msg);
    }
}

pub fn handler(path: &Path) -> Handler<'static> {
    let ctx = Mutex::new(Context::new(path));
    Box::new(move |record| {
        let mut ctx = ctx.lock().unwrap();
        ctx.emit(record.formatted().as_bytes());
    })
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use self::tempdir::TempDir;

    #[test]
    fn test_file() {
        let dir = TempDir::new("wp-f").unwrap();
        let path = dir.path().join("logs").join("test.log");
        let mut ctx = super::Context::new(&path);

        assert!(path.exists());
        assert_eq!(path.metadata().unwrap().len(), 0);
        ctx.emit("x".as_bytes());
        assert_eq!(path.metadata().unwrap().len(), 1);
        ctx.emit("x".as_bytes());
        assert_eq!(path.metadata().unwrap().len(), 2);
    }
}
