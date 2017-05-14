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

// TODO: use pub(crate) when stabilized (should in v1.18)
// https://github.com/rust-lang/rust/issues/32409
#[doc(hidden)]
pub fn emit(formatted: &String) {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    stderr.write(formatted.as_bytes()).unwrap();
}

/// Pushes formatted log record into stderr.
pub fn handler() -> Handler<'static> {
    Box::new(|record| {
        emit(record.formatted().deref());
    })
}
