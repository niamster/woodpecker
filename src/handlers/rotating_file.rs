use std::fs::{File, OpenOptions, create_dir_all, rename};
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use std::io::Write;

use logger::Handler;

struct Context {
    path: PathBuf,
    range: Vec<PathBuf>,
    size: u64,
    current: u64,
    file: File,
}

impl Context {
    fn open(path: &Path) -> File {
        OpenOptions::new().append(true).create(true).open(path).unwrap()
    }

    fn new(path: &Path, count: usize, size: u64) -> Self {
        let file = Self::open(path);
        assert!(size > 0);
        assert!(count > 0);
        Context {
            path: path.into(),
            range: (0..count-1).rev().map(|r| PathBuf::from(format!("{}.{}", path.display(), r))).collect(),
            size: size,
            current: file.metadata().unwrap().len(),
            file: file,
        }
    }

    fn emit(&mut self, msg: &[u8]) {
        self.file.write(msg).unwrap();
        self.current += msg.len() as u64;
        if self.current >= self.size {
            drop(&self.file);
            let rlen = self.range.len();
            for i in 1..rlen {
                let old = &self.range[i];
                if old.exists() {
                    let _ = rename(old, &self.range[i-1]);
                }
            }
            let _ = rename(&self.path, &self.range[rlen-1]);
            self.file = Self::open(&self.path);
            self.current = 0;
        }
    }
}

pub fn handler(path: &Path, count: usize, size: u64) -> Handler<'static> {
    match path.parent() {
        Some(dir) => create_dir_all(dir).unwrap(),
        None => {},
    };
    let ctx = Mutex::new(Context::new(path, count, size));
    Box::new(move |record| {
        let mut ctx = ctx.lock().unwrap();
        ctx.emit(record.formatted().as_bytes());
    })
}
