// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use record::Record;

pub fn formatter(record: &Record) -> Box<String> {
    Box::new(format!(
        concat!("|{}| {} {}", wp_separator!(), "{}:{} {}\n"),
        record.level,
        record.ts_utc(),
        record.module,
        record.file,
        record.line,
        record.msg(),
    ))
}
