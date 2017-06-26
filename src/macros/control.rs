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

/// Sets the log level.
///
/// Sets global log level if called without the arguments or
/// according to the specified path otherwise.
///
/// Optionally the ranges of the lines within the file could
/// be given.
/// In this case the full path to the file must be provided.
///
/// The logger spec might also be either [env_logger](https://doc.rust-lang.org/log/env_logger) spec or
/// a JSON string which defines global and per-module log level.
///
/// See documentation of the [spec](spec/index.html) module for the details.
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
/// fn main() {
///     wp_init!();
///
///     let logger = "foo::bar";
///
///     wp_set_level!(wp::LogLevel::INFO).unwrap();
///     wp_set_level!(wp::LogLevel::CRITICAL, logger).unwrap();
///
///     assert_eq!(wp_get_level!(^), wp::LogLevel::INFO);
///     assert_eq!(wp_get_level!(), wp::LogLevel::INFO);
///     assert_eq!(wp_get_level!(logger), wp::LogLevel::CRITICAL);
///
///     wp_set_level!(wp::LogLevel::WARN, this_file!(), [(line!() + 2, wp::EOF)]).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::INFO);
///     assert_eq!(wp_get_level!(), wp::LogLevel::WARN);
///
///     wp_set_level!(wp::LogLevel::CRITICAL, this_file!()).unwrap();
///     let ranges = vec![
///         (wp::BOF.into(), line!() + 4).into(),
///         (line!() + 5, wp::EOF.into()).into()
///     ];
///     wp_set_level!(wp::LogLevel::WARN, this_file!(), ranges).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::WARN);
///     assert_eq!(wp_get_level!(), wp::LogLevel::CRITICAL);
///     assert_eq!(wp_get_level!(), wp::LogLevel::WARN);
///
///     wp_set_level!(wp::LogLevel::TRACE, this_file!(), [(wp::BOF, wp::EOF)]).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::TRACE);
///
///     let spec = format!(r#"{{
///                     "level": "debug",
///                     "modules": [
///                         {{
///                             "path": "foo"
///                         }},
///                         {{
///                             "path": "{}",
///                             "level": "notice"
///                         }},
///                         {{
///                             "path": "{}",
///                             "level": "critical",
///                             "lines": [[{}, {}]]
///                         }}
///                     ]
///                 }}"#,
///             this_file!(), this_file!(), line!() + 4, line!() + 100);
///     wp_set_level!(spec(&spec)).unwrap();
///     assert_eq!(wp_get_level!(^), wp::LogLevel::DEBUG);
///     assert_eq!(wp_get_level!(), wp::LogLevel::NOTICE);
///     assert_eq!(wp_get_level!("foo"), wp::LogLevel::TRACE);
///     assert_eq!(wp_get_level!(), wp::LogLevel::CRITICAL);
///
///     let spec = format!("critical,{},foo=info", this_file!());
///     wp_set_level!(spec(&spec)).unwrap();
///     assert_eq!(wp_get_level!(^), wp::LogLevel::CRITICAL);
///     assert_eq!(wp_get_level!(), wp::LogLevel::TRACE);
///     assert_eq!(wp_get_level!("foo"), wp::LogLevel::INFO);
/// }
///
/// ```
#[macro_export]
macro_rules! wp_set_level {
    ($level:expr, $logger:expr, [$(($from:expr, $to:expr)),*]) => {{
        let mut lranges = Vec::new();
        $(
            let (from, to): (u32, u32) = ($from.into(), $to.into());
            lranges.push((from, to));
        )*;
        match $crate::line_range::from_vtuple(&lranges) {
            Ok(lranges) => {
                wp_set_level!($level, $logger, lranges)
            },
            Err(err) => {
                let err: Result<(), String> = Err(format!("{:?}", err));
                err
            }
        }
    }};

    ($level:expr, $logger:expr, $lranges:expr) => {{
        match $crate::line_range::spec($level, &$lranges) {
            Ok(lranges) => {
                if lranges.is_empty() {
                    __wp_write_root!(set_level($logger, $level))
                } else {
                    __wp_write_root!(set_level_ranges($logger, lranges))
                }
            },
            Err(err) => {
                let err: Result<(), String> = Err(format!("{:?}", err));
                err
            },
        }
    }};

    ($level:expr, $logger:expr) => {{
        __wp_write_root!(set_level($logger, $level))
    }};

    (spec($spec:expr)) => {{
        match $crate::spec::parse($spec) {
            Ok(spec) => {
                if let Some(level) = spec.level {
                    let _ = wp_set_level!(level);
                }
                || -> Result<(), String> {
                    for module in &spec.modules {
                        wp_set_level!(module.level, &module.path, &module.lranges)?;
                    }
                    Ok(())
                }()
            },
            Err(err) => Err(format!("{:?}", err))
        }
    }};

    ($level:expr) => {{
        $crate::global::set_loggers(false);
        __wp_write_root!(reset_loggers());
        $crate::global::set_level($level);
        let ok: Result<(), String> = Ok(());
        ok
    }};
}

/// Gets the log level.
///
/// Returns global log level if called with `^` as an argument.
///
/// Returns log level according to the position (current path)
/// if called without any argument.
///
/// Returns log level for the requested path when it's passed as an argument.
///
/// The log levels are hierarchical.
///
/// It means that if, for example, there's a rule that states that the module `foo::bar`
/// has log level `WARN`, then all submodules inherit this log level.
/// At the same time another rule may override the inherited level.
/// For example, `foo::bar::qux@xyz.rs` has log level `DEBUG`.
///
/// If there's no exact match the rules of the parent are applied.
///
/// # Example
///
/// ```rust
/// #[macro_use]
/// extern crate woodpecker;
/// use woodpecker as wp;
///
/// fn main() {
///     wp_init!();
///
///     let logger = "foo::bar";
///
///     assert_eq!(wp_get_level!(^), wp::LogLevel::WARN);
///     assert_eq!(wp_get_level!(logger), wp::LogLevel::WARN);
///
///     wp_set_level!(wp::LogLevel::INFO).unwrap();
///     wp_set_level!(wp::LogLevel::CRITICAL, logger).unwrap();
///
///     assert_eq!(wp_get_level!(^), wp::LogLevel::INFO);
///     assert_eq!(wp_get_level!(logger), wp::LogLevel::CRITICAL);
///
///     // Since the logs follow the hierarchy following statements are valid.
///     assert_eq!(wp_get_level!("foo::bar::qux"), wp::LogLevel::CRITICAL);
///     assert_eq!(wp_get_level!("foo"), wp::LogLevel::INFO);
///
///     wp_set_level!(wp::LogLevel::CRITICAL, this_module!()).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::CRITICAL);
///
///     wp_set_level!(wp::LogLevel::DEBUG, this_file!()).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::DEBUG);
///
///     assert_eq!(wp_get_level!(^), wp::LogLevel::INFO);
///     assert_eq!(wp_get_level!(this_module!()), wp::LogLevel::CRITICAL);
///     assert_eq!(wp_get_level!(this_file!()), wp::LogLevel::DEBUG);
/// }
///
/// ```
#[macro_export]
macro_rules! wp_get_level {
    (^) => {{
        $crate::global::get_level()
    }};

    () => {{
        if $crate::global::has_loggers() {
            let path = this_file!();
            __wp_read_root!(get_level(path, line!()))
        } else {
            $crate::global::get_level()
        }
    }};

    ($logger:expr) => {{
        if $crate::global::has_loggers() {
            __wp_read_root!(get_level($logger, $crate::EOF.into()))
        } else {
            $crate::global::get_level()
        }
    }};
}

/// Registers a log record handler.
///
/// The handler takes a log record as an argument and pushes it into a custom sink.
///
/// By default if no log handler is registered `woodpecker` emits
/// log records into `stdout`.
///
/// If at least one handler is registered than the `stdout` handler
/// must be registered explicitly if it's still desired.
///
/// See the definition of the [`Handler`](handlers/type.Handler.html) type for the details.
///
/// # Example
/// In this example string "foo" will be logged three times into `stdout`
/// but only one caught by the log handler.
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
///     wp_init!();
///
///     warn!("foo");
///     let out = Arc::new(Mutex::new(String::new()));
///     {
///         wp_register_handler!(wp::handlers::stdout::handler());
///         warn!("foo");
///         let out = out.clone();
///         wp_register_handler!(Box::new(move |record| {
///             out.lock().unwrap().push_str(record.msg().deref());
///         }));
///
///         warn!("foo");
///     }
///     if cfg!(feature = "test-thread-log") {
///         wp::sync();
///     }
///     assert_eq!(*out.lock().unwrap(), "foo".to_string());
/// }
///
/// ```
#[macro_export]
macro_rules! wp_register_handler {
    ($handler:expr) => {{
        __wp_write_root!(handler($handler));
    }};
}

/// Sets a log record formatter.
///
/// A [default](formatters/default/fn.formatter.html) formatter is used if not set explicitly.
///
/// The formatter function takes a log record as an argument and returns a string
/// that will be used as a return value of [Record::formatted](record/trait.Record.html#tymethod.formatted) function.
///
/// See the definition of the [Formatter](formatters/type.Formatter.html) type for the details.
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
///     wp_init!();
///
///     wp_set_formatter!(Box::new(|record| {
///         format!("{}:{}", record.level(), record.msg())
///     }));
///     let out = Arc::new(Mutex::new(String::new()));
///     {
///         let out = out.clone();
///         wp_register_handler!(Box::new(move |record| {
///             out.lock().unwrap().push_str(record.formatted().deref());
///         }));
///
///         warn!("foo");
///     }
///     if cfg!(feature = "test-thread-log") {
///         wp::sync();
///     }
///     assert_eq!(*out.lock().unwrap(), "WARN:foo".to_string());
/// }
///
/// ```
#[macro_export]
macro_rules! wp_set_formatter {
    ($formatter:expr) => {{
        __wp_write_root!(formatter($formatter));
    }};
}
