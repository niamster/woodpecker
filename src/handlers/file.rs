use std::fs::{OpenOptions, create_dir_all};
use std::sync::Mutex;
use std::path::Path;
use std::io::Write;

use logger::Handler;

pub fn handler(path: &Path) -> Handler<'static> {
    match path.parent() {
        Some(dir) => create_dir_all(dir).unwrap(),
        None => {},
    };
    let file = Mutex::new(
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .unwrap()
    );
    Box::new(move |record| {
        let mut file = file.lock().unwrap();
        file.write(record.formatted().as_bytes()).unwrap();
    })
}
