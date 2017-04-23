// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::{self, Write};
use std::ops::Deref;

use handlers::Handler;

pub fn emit(formatted: &String) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write(formatted.as_bytes()).unwrap();
}

pub fn handler() -> Handler<'static> {
    Box::new(|record| {
        emit(record.formatted().deref());
    })
}
