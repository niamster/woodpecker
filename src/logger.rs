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

extern crate parking_lot;
use self::parking_lot::RwLock;

extern crate time;

use std::mem;
use std::sync::{Arc, Once, ONCE_INIT};
use std::sync::atomic::{AtomicIsize, AtomicBool, Ordering, ATOMIC_ISIZE_INIT, ATOMIC_BOOL_INIT};
use std::collections::LinkedList;
use std::fmt;

// TODO: per-thread log

use levels::LogLevel;
use record::Record;
use formatters::Formatter;
use handlers::Handler;

pub static LOG_LEVEL: AtomicIsize = ATOMIC_ISIZE_INIT;
pub static HAS_SUBLOGGERS: AtomicBool = ATOMIC_BOOL_INIT;

pub struct RootLogger<'a> {
    loggers: Vec<(String, LogLevel)>,
    formatter: Formatter<'a>,
    handlers: LinkedList<Handler<'a>>,
}

impl<'a> RootLogger<'a> {
    pub fn new() -> Self {
        RootLogger {
            loggers: Vec::new(),
            formatter: Box::new(::formatters::default::formatter),
            handlers: LinkedList::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn handler(&mut self, handler: Handler<'a>) {
        self.handlers.push_front(handler);
    }

    pub fn formatter(&mut self, formatter: Formatter<'a>) {
        self.formatter = formatter;
    }

    pub fn set_level(&mut self, path: &str, level: LogLevel) {
        self.loggers.retain(|&(ref name, _)| !(name.len() > path.len() && name.starts_with(path)));
        match self.loggers.binary_search_by(|&(ref k, _)| path.cmp(&k)) {
            Ok(pos) => {
                let (_, ref mut olevel) = self.loggers[pos];
                *olevel = level;
            },
            Err(pos) => {
                self.loggers.insert(pos, (path.to_string(), level));
            },
        };
    }

    #[inline(always)]
    pub fn get_level(&self, path: &str) -> LogLevel {
        match self.loggers.binary_search_by(|&(ref k, _)| path.cmp(&k)) {
            Ok(pos) => {
                let &(_, level) = &self.loggers[pos];
                level
            },
            Err(pos) => {
                let &(ref name, level) = &self.loggers[if pos > 0 { pos - 1} else { pos }];
                if path.starts_with(name) {
                    level
                } else {
                    global_get_level()
                }
            },
        }
    }

    pub fn log(&self, level: LogLevel, module: &'static str, file: &'static str, line: u32, args: fmt::Arguments) {
        let record = Record::new(level, module, file, line, time::get_time(), args, &self.formatter);
        if self.handlers.is_empty() {
            ::handlers::stdout::emit(&record.formatted());
        } else {
            for h in &self.handlers {
                h(&record);
            }
        }
    }
}

#[inline(always)]
pub fn root() -> Arc<RwLock<RootLogger<'static>>> {
    static mut LOGGERS: *const Arc<RwLock<RootLogger<'static>>> = 0 as *const Arc<RwLock<RootLogger>>;
    static ONCE: Once = ONCE_INIT;
    unsafe {
        ONCE.call_once(|| {
            LOG_LEVEL.store(isize::from(LogLevel::WARN), Ordering::Relaxed);
            let root = Arc::new(RwLock::new(RootLogger::new()));
            LOGGERS = mem::transmute(Box::new(root));
        });

        (*LOGGERS).clone()
    }
}

pub fn reset() {
    let root = root();
    let mut root = root.write();
    global_set_level(LogLevel::WARN);
    global_set_loggers(false);
    root.reset();
}

#[inline(always)]
pub fn global_get_level() -> LogLevel {
    LogLevel::from(LOG_LEVEL.load(Ordering::Relaxed))
}

pub fn global_set_level(level: LogLevel) {
    LOG_LEVEL.store(level.into(), Ordering::Relaxed);
}

#[inline(always)]
pub fn global_has_loggers() -> bool {
    HAS_SUBLOGGERS.load(Ordering::Relaxed)
}

pub fn global_set_loggers(value: bool) {
    HAS_SUBLOGGERS.store(value, Ordering::Relaxed);
}

#[macro_export]
macro_rules! wp_set_level {
    ($logger:expr, $level:expr) => {{
        __wp_logger_is_string!($logger);
        let root = $crate::logger::root();
        let mut root = root.write();
        root.set_level(&$logger.to_string(), $level);
        $crate::logger::global_set_loggers(true);
    }};
    ($level:expr) => {{
        $crate::logger::global_set_level($level);
    }};
}

#[macro_export]
macro_rules! wp_get_level {
    () => {{
        $crate::logger::global_get_level()
    }};
    ($logger:expr) => {{
        __wp_logger_is_string!($logger);
        if $crate::logger::global_has_loggers() {
            let root = $crate::logger::root();
            let level = root.read().get_level(&$logger.to_string());
            level
        } else {
            $crate::logger::global_get_level()
        }
    }};
}

#[macro_export]
macro_rules! __wp_write_action {
    ($func:ident($arg:expr)) => {{
        let root = $crate::logger::root();
        root.write().$func($arg);
    }};
}

#[macro_export]
macro_rules! wp_set_handler {
    ($handler:expr) => {{
        __wp_write_action!(handler($handler));
    }};
}

#[macro_export]
macro_rules! wp_set_formatter {
    ($formatter:expr) => {{
        __wp_write_action!(formatter($formatter));
    }};
}

#[macro_export]
macro_rules! wp_separator {
    () => ("@")
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {{
        if $crate::logger::global_has_loggers() {
            let path = this_file!();
            let root = $crate::logger::root();
            let root = root.read();
            if root.get_level(path) <= $level {
                root.log($level, module_path!(), file!(), line!(), format_args!($($arg)*));
            }
        } else {
            if $crate::logger::global_get_level() <= $level {
                let root = $crate::logger::root();
                let root = root.read();
                root.log($level, module_path!(), file!(), line!(), format_args!($($arg)*));
            }
        }
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::DEBUG, $($arg)*);
    };
}

#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::VERBOSE, $($arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::INFO, $($arg)*);
    };
}

#[macro_export]
macro_rules! notice {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::NOTICE, $($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::WARN, $($arg)*);
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::ERROR, $($arg)*);
    };
}

#[macro_export]
macro_rules! critical {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::CRITICAL, $($arg)*);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    use levels::LEVELS;

    use std::sync::Mutex;
    use std::ops::Deref;
    use std::thread;
    use std::panic;

    // NOTE: the test must not run in //
    fn run_test<T>(test: T) where T: FnOnce(Arc<Mutex<String>>) -> () + panic::UnwindSafe {
        struct TContext {
            lock: Mutex<u32>,
        };
        static mut CONTEXT: *const Arc<TContext> = 0 as *const Arc<TContext>;
        static ONCE: Once = ONCE_INIT;

        let context = unsafe {
            ONCE.call_once(|| {
                let context = Arc::new(TContext {
                    lock: Mutex::new(0),
                });
                CONTEXT = mem::transmute(Box::new(context));
            });

            (*CONTEXT).clone()
        };

        let lock = context.lock.lock().unwrap();

        let result = panic::catch_unwind(|| {
            reset();
            let out = Arc::new(Mutex::new(String::new()));
            {
                let out = out.clone();
                wp_set_handler!(Box::new(move |record| {
                    out.lock().unwrap().push_str(record.formatted().deref());
                }));
            }
            (test)(out.clone());
        });

        drop(lock);

        if let Err(err) = result {
            panic::resume_unwind(err);
        }
    }

    #[test]
    fn test_logger_default() {
        run_test(|_| {
            // Default level is WARN
            assert_eq!(wp_get_level!(), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo"), LogLevel::WARN);

            wp_set_level!(LogLevel::INFO);
            assert_eq!(wp_get_level!(), LogLevel::INFO);
            assert_eq!(wp_get_level!("foo"), LogLevel::INFO);
        });
    }

    #[test]
    #[should_panic(expected = "Unsupported log level -1000")]
    fn test_logger_level() {
        LogLevel::from(-1000);
    }

    #[test]
    #[should_panic(expected = "Logger name must be a string")]
    fn test_logger_type_string_0() {
        wp_get_level!(42);
    }

    #[test]
    #[should_panic(expected = "Logger name must be a string")]
    fn test_logger_type_string_1() {
        wp_set_level!(42, LogLevel::INFO);
    }

    #[test]
    fn test_logger_basic() {
        run_test(|buf| {
            for l in LEVELS.iter() {
                wp_set_level!(*l);
                assert_eq!(*l, wp_get_level!());

                for l in LEVELS.iter() {
                    log!(*l, "msg");
                    let mut output = buf.lock().unwrap();
                    if *l >= wp_get_level!() {
                        assert!(output.as_str().contains("msg"));
                        assert!(output.as_str().contains(&l.to_string()));
                    } else {
                        assert!(output.is_empty());
                    }
                    output.clear();
                }
            }

            wp_set_formatter!(Box::new(|record| {
                Box::new(format!(
                    "{}:{}",
                    record.level,
                    record.msg(),
                ))
            }));

            let logger = "woodpecker";
            wp_set_level!(LogLevel::WARN);
            wp_set_level!(logger, LogLevel::VERBOSE);
            assert_eq!(wp_get_level!(), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo"), LogLevel::WARN);
            assert_eq!(wp_get_level!(logger), LogLevel::VERBOSE);

            {
                let mut output = buf.lock().unwrap();
                output.clear();
                drop(output);
                verbose!("msg");
                debug!("msg");
                let output = buf.lock().unwrap();
                assert_eq!(output.as_str(), "VERBOSE:msg");
            }

            wp_set_level!(LogLevel::CRITICAL);

            let logger = this_module!();
            wp_set_level!(logger, LogLevel::ERROR);
            assert_eq!(wp_get_level!(), LogLevel::CRITICAL);
            assert_eq!(wp_get_level!(logger), LogLevel::ERROR);

            let logger = this_file!();
            wp_set_level!(logger, LogLevel::NOTICE);
            assert_eq!(wp_get_level!(), LogLevel::CRITICAL);
            assert_eq!(wp_get_level!(logger), LogLevel::NOTICE);

            {
                let mut output = buf.lock().unwrap();
                output.clear();
                drop(output);
                notice!("msg");
                verbose!("msg");
                let output = buf.lock().unwrap();
                assert_eq!(output.as_str(), "NOTICE:msg");
            }
        });
    }

    #[test]
    fn test_logger_handler() {
        run_test(|buf| {
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                wp_set_handler!(Box::new(move |record| {
                    out.write().push_str(record.msg().deref());
                    out.write().push_str("|");
                }));

                wp_set_level!(LogLevel::INFO);
                info!("msg");
                debug!("foo");
            }
            assert_eq!(buf.lock().unwrap().split("msg").count(), 2);
            assert_eq!(*out.read(), "msg|".to_string());
        });
    }

    #[test]
    fn test_logger_formatter() {
        run_test(|_| {
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                wp_set_handler!(Box::new(move |record| {
                    out.write().push_str(record.formatted().deref());
                }));
                wp_set_formatter!(Box::new(|record| {
                    Box::new(format!(
                        "{}:{}|",
                        record.level,
                        record.msg(),
                    ))
                }));

                wp_set_level!(LogLevel::INFO);
                info!("msg");
                debug!("foo");
            }
            assert_eq!(*out.read(), "INFO:msg|".to_string());
        });
    }

    #[test]
    fn test_logger_threads() {
        run_test(|_| {
            let thqty = 100;
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                wp_set_handler!(Box::new(move |record| {
                    out.write().push_str(record.formatted().deref());
                }));
                wp_set_formatter!(Box::new(move |record| {
                    Box::new(format!(
                        "{}:{}",
                        record.level,
                        record.msg(),
                    ))
                }));

                let mut threads = Vec::new();
                wp_set_level!(LogLevel::INFO);
                for idx in 0..thqty {
                    threads.push(thread::spawn(move || {
                        thread::yield_now();
                        if idx % 2 == 0 {
                            info!("{}", idx);
                        }
                        debug!("{}", idx);
                    }));
                }

                assert_eq!(thqty, threads.len());
                for th in threads {
                    th.join().unwrap();
                }
            }
            let sum = out.read().split("INFO:").
                filter(|val| !val.is_empty()).
                fold(0, |acc, ref val| {
                    acc + val.parse::<u32>().unwrap()
                });
            let expected = (0..100).filter(|x| x % 2 == 0).sum();
            assert_eq!(sum, expected);
        });
    }
}
