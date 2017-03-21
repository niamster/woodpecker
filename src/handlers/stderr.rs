use std::io::{self, Write};
use std::ops::Deref;

use logger::Handler;

pub fn emit(formatted: &String) {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    stderr.write(formatted.as_bytes()).unwrap();
}

pub fn handler() -> Handler<'static> {
    Box::new(|record| {
        emit(record.formatted().deref());
    })
}
