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

use std::io::{self, Write};
use std::ops::Deref;

use handlers::Handler;

pub(crate) fn emit(formatted: &str) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = stdout.write_all(formatted.as_bytes());
}

/// Pushes formatted log record into stdout.
pub fn handler() -> Handler {
    Box::new(|record| {
        emit(record.formatted().deref());
    })
}
