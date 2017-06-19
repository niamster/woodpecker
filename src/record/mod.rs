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

extern crate chrono;
use self::chrono::prelude::*;

extern crate time;

use std::sync::Arc;

use levels::LogLevel;

/// Log record that holds information where log was recorded
/// and the message details.
pub trait Record: Send + Sync {
    /// Log level of the record.
    fn level(&self) -> LogLevel;

    /// Module path.
    fn module(&self) -> &'static str;

    /// File path.
    fn file(&self) -> &'static str;

    /// Line number.
    fn line(&self) -> u32;

    /// Timestamp.
    fn ts(&self) -> time::Timespec;

    /// Returns user log message as a formatted string.
    fn msg(&self) -> Arc<String>;

    /// Returns record formatted as a string using given formatter.
    fn formatted(&self) -> Arc<String>;

    /// Returns timestamp in UTC.
    fn ts_utc(&self) -> Arc<DateTime<UTC>>;
}

#[doc(hidden)]
pub mod imp;
