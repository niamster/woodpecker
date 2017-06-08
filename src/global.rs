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

use std::sync::atomic::{AtomicIsize, AtomicBool, Ordering,
                        ATOMIC_ISIZE_INIT, ATOMIC_BOOL_INIT};

use levels::LogLevel;

static LOG_LEVEL: AtomicIsize = ATOMIC_ISIZE_INIT;
static HAS_SUBLOGGERS: AtomicBool = ATOMIC_BOOL_INIT;

#[doc(hidden)]
#[inline(always)]
pub fn get_level() -> LogLevel {
    LogLevel::from(LOG_LEVEL.load(Ordering::Relaxed))
}

#[doc(hidden)]
pub fn set_level(level: LogLevel) {
    LOG_LEVEL.store(level.into(), Ordering::Relaxed);
}

#[doc(hidden)]
#[inline(always)]
pub fn has_loggers() -> bool {
    HAS_SUBLOGGERS.load(Ordering::Relaxed)
}

#[doc(hidden)]
pub fn set_loggers(value: bool) {
    HAS_SUBLOGGERS.store(value, Ordering::Relaxed);
}
