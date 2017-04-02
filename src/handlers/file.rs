use std::fs::{OpenOptions, create_dir_all};
use std::sync::Mutex;
use std::path::Path;
use std::io::Write;
use std::cell::RefCell;
use std::sync::Arc;

use logger::Handler;

pub fn handler(path: &Path) -> Handler<'static> {
    match path.parent() {
        Some(dir) => create_dir_all(dir).unwrap(),
        None => {},
    };
    let file = Arc::new(Mutex::new(RefCell::new(
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .unwrap()
    )));
    Box::new(move |record| {
        let file = file.clone();
        let file = file.lock().unwrap();
        let mut file = file.borrow_mut();
        file.write(record.formatted().as_bytes()).unwrap();
    })
}
