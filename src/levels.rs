// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    VERBOSE,
    INFO,
    NOTICE,
    WARN,
    ERROR,
    CRITICAL,
}

impl From<LogLevel> for isize {
    fn from(orig: LogLevel) -> isize {
        match orig {
            LogLevel::TRACE => -30,
            LogLevel::DEBUG => -20,
            LogLevel::VERBOSE => -10,
            LogLevel::INFO => 0,
            LogLevel::NOTICE => 10,
            LogLevel::WARN => 20,
            LogLevel::ERROR => 30,
            LogLevel::CRITICAL => 40,
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
            _   => panic!("Unsupported log level {}", orig),
        }
    }
}

pub const LEVELS: [LogLevel; 8] = [
    LogLevel::TRACE,
    LogLevel::DEBUG,
    LogLevel::VERBOSE,
    LogLevel::INFO,
    LogLevel::NOTICE,
    LogLevel::WARN,
    LogLevel::ERROR,
    LogLevel::CRITICAL
];

impl fmt::Display for LogLevel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LogLevel::TRACE => write!(f, "TRACE"),
            &LogLevel::DEBUG => write!(f, "DEBUG"),
            &LogLevel::VERBOSE => write!(f, "VERBOSE"),
            &LogLevel::INFO => write!(f, "INFO"),
            &LogLevel::NOTICE => write!(f, "NOTICE"),
            &LogLevel::WARN => write!(f, "WARN"),
            &LogLevel::ERROR => write!(f, "ERROR"),
            &LogLevel::CRITICAL => write!(f, "CRITICAL"),
        }
    }
}
