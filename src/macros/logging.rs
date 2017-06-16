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

/// The main log entry.
///
/// Prepares and emits a log record if requested log [level](levels/enum.LogLevel.html) is greater or equal
/// according to the log level.
///
/// If log level is not specified then the log is emitted unconditionally.
///
/// If, for example, the hierarchy rules deduce that the log level at the current position is
/// [WARN](levels/enum.LogLevel.html) then the logs for
/// the levels [WARN](levels/enum.LogLevel.html) and above([ERROR](levels/enum.LogLevel.html) and
/// [CRITICAL](levels/enum.LogLevel.html)) are emitted.
/// The logs for the levels below [WARN](levels/enum.LogLevel.html)
/// (such as [NOTICE](levels/enum.LogLevel.html), [INFO](levels/enum.LogLevel.html), etc.) are dropped.
///
/// See documentation for the [wp_get_level](macro.wp_get_level.html)
/// for more details on the log level hierarchy.
///
/// # Example
///
/// ```rust
/// #[macro_use]
/// extern crate woodpecker;
/// use woodpecker as wp;
///
/// use std::sync::{Arc, Mutex};
/// use std::ops::Deref;
///
/// fn main() {
///     let msg = "I'm always visible";
///
///     wp::init();
///
///     wp_set_level!(wp::LogLevel::CRITICAL).unwrap();
///     let out = Arc::new(Mutex::new(String::new()));
///     {
///         let out = out.clone();
///         wp_register_handler!(Box::new(move |record| {
///             out.lock().unwrap().push_str(record.msg().deref());
///         }));
///
///         log!(">{}<", msg);
///         warn!("foo");
///         in_trace!({
///             log!("Not seen though");
///         });
///     }
///     if cfg!(feature = "test-thread-log") {
///         wp::sync();
///     }
///     assert_eq!(*out.lock().unwrap(), format!(">{}<", msg));
/// }
///
/// ```
#[macro_export]
macro_rules! log {
    ($level:expr => $($arg:tt)*) => {{
        use $crate::record::imp::RecordMeta;
        static RECORD: RecordMeta = RecordMeta {
            level: $level,
            module: this_module!(),
            file: file!(),
            line: line!(),
        };
        if $crate::global::has_loggers() {
            let path = this_file!();
            let root = $crate::logger::ROOT.read();
            if root.get_level(path, line!()) <= $level {
                root.log(&RECORD, format_args!($($arg)*));
            }
        } else {
            if $crate::global::get_level() <= $level {
                __wp_read_root!(log(&RECORD, format_args!($($arg)*)));
            }
        }
    }};

    ($($arg:tt)*) => {{
        use $crate::record::imp::RecordMeta;
        static RECORD: RecordMeta = RecordMeta {
            level: $crate::LogLevel::LOG,
            module: this_module!(),
            file: file!(),
            line: line!(),
        };
        __wp_read_root!(log(&RECORD, format_args!($($arg)*)));
    }};
}

/// Produces log record for the `trace` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::TRACE => $($arg)*);
    };
}

/// Executes the code only for the `trace` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_trace {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::TRACE {
            $block;
        }
    }
}

/// Produces log record for the `debug` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::DEBUG  => $($arg)*);
    };
}

/// Executes the code only for the `debug` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_debug {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::DEBUG {
            $block;
        }
    }
}

/// Produces log for the `verbose` level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::VERBOSE => $($arg)*);
    };
}

/// Executes the code only for the `verbose` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_verbose {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::VERBOSE {
            $block;
        }
    }
}

/// Produces log record for the `info` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::INFO => $($arg)*);
    };
}

/// Executes the code only for the `info` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_info {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::INFO {
            $block;
        }
    }
}

/// Produces log record for the `notice` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! notice {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::NOTICE => $($arg)*);
    };
}

/// Executes the code only for the `notice` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_notice {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::NOTICE {
            $block;
        }
    }
}

/// Produces log record for the `warn` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::WARN => $($arg)*);
    };
}

/// Executes the code only for the `warn` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_warn {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::WARN {
            $block;
        }
    }
}

/// Produces log record for the `error` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::ERROR => $($arg)*);
    };
}

/// Executes the code only for the `error` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_error {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::ERROR {
            $block;
        }
    }
}

/// Produces log record for the `critical` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! critical {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::CRITICAL => $($arg)*);
    };
}

/// Executes the code only for the `critical` log level.
///
/// See the [log](macro.log.html) macro for the details.
#[macro_export]
macro_rules! in_critical {
    ($block:block) => {
        if wp_get_level!() <= $crate::LogLevel::CRITICAL {
            $block;
        }
    }
}
