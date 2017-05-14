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

/// Type of the log handlers function.
///
/// Pushes a log record into a log sink.
pub type Handler<'a> = Box<Fn(&Record) + 'a + Send + Sync>;

/// Stdout log handler.
pub mod stdout;
/// Stderr log handler.
pub mod stderr;
/// File log handler.
pub mod file;
/// Rotating file log handler.
pub mod rotating_file;
