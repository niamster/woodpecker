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

use record::Record;

/// A default log formatter which outputs the record in as a line.
///
/// # Example:
///
/// ```ignore
/// |<log-level>| time module@file:line <message>
/// ```
pub fn formatter(record: &Record) -> String {
    format!(
        concat!("|{}| {} {}", wp_separator!(), "{}:{} {}\n"),
        record.level(),
        record.ts_utc(),
        record.module(),
        record.file(),
        record.line(),
        record.msg(),
    )
}
