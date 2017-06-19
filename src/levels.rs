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

use std::fmt;
use std::str::FromStr;
use std::collections::HashMap;

/// The logging levels.
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum LogLevel {
    #[doc(hidden)]
    UNSUPPORTED,
    /// Log all messages.
    TRACE,
    /// Log only debug messages and above.
    DEBUG,
    /// Log only verbose messages and above.
    VERBOSE,
    /// Log only info messages and above.
    INFO,
    /// Log only notice messages and above.
    NOTICE,
    /// Log only warn messages and above.
    WARN,
    /// Log only error messages and above.
    ERROR,
    /// Log only critical messages.
    CRITICAL,
    #[doc(hidden)]
    LOG,
}

impl From<LogLevel> for isize {
    fn from(orig: LogLevel) -> isize {
        match orig {
            LogLevel::UNSUPPORTED => -10000,
            LogLevel::TRACE => -30,
            LogLevel::DEBUG => -20,
            LogLevel::VERBOSE => -10,
            LogLevel::INFO => 0,
            LogLevel::NOTICE => 10,
            LogLevel::WARN => 20,
            LogLevel::ERROR => 30,
            LogLevel::CRITICAL => 40,
            LogLevel::LOG => 50,
        }
    }
}

impl From<isize> for LogLevel {
    #[inline(always)]
    fn from(orig: isize) -> LogLevel {
        match orig {
            -30 => LogLevel::TRACE,
            -20 => LogLevel::DEBUG,
            -10 => LogLevel::VERBOSE,
            0   => LogLevel::INFO,
            10  => LogLevel::NOTICE,
            20  => LogLevel::WARN,
            30  => LogLevel::ERROR,
            40  => LogLevel::CRITICAL,
            50  => LogLevel::LOG,
            _   => LogLevel::UNSUPPORTED,
        }
    }
}

// NOTE: `LOG` level should not be included here
#[doc(hidden)]
pub(crate) const LEVELS: [LogLevel; 8] = [
    LogLevel::TRACE,
    LogLevel::DEBUG,
    LogLevel::VERBOSE,
    LogLevel::INFO,
    LogLevel::NOTICE,
    LogLevel::WARN,
    LogLevel::ERROR,
    LogLevel::CRITICAL
];

lazy_static! {
    static ref LMAP: HashMap<String, LogLevel> = {
        let mut levels = HashMap::with_capacity(LEVELS.len());
        for level in &LEVELS {
            levels.insert(level.to_string().to_uppercase(), *level);
        }
        levels
    };
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(level) = LMAP.get(&s.to_uppercase()) {
            Ok(*level)
        } else {
            Err(())
        }
    }
}

impl fmt::Display for LogLevel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogLevel::UNSUPPORTED => write!(f, "UNSUPPORTED"),
            LogLevel::TRACE => write!(f, "TRACE"),
            LogLevel::DEBUG => write!(f, "DEBUG"),
            LogLevel::VERBOSE => write!(f, "VERBOSE"),
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::NOTICE => write!(f, "NOTICE"),
            LogLevel::WARN => write!(f, "WARN"),
            LogLevel::ERROR => write!(f, "ERROR"),
            LogLevel::CRITICAL => write!(f, "CRITICAL"),
            LogLevel::LOG => write!(f, "LOG"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_level() {
        assert_eq!(LogLevel::from(0), LogLevel::INFO);

        assert_eq!(LogLevel::from(-1), LogLevel::UNSUPPORTED);
        assert_eq!(LogLevel::from(-1000), LogLevel::UNSUPPORTED);
    }
}
