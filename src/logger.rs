// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate chrono;
use self::chrono::prelude::*;

use std::mem;
use std::path;
use std::sync::{Arc, RwLock, Once, ONCE_INIT};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fmt;
use std::fmt::Write;
use std::any::{Any, TypeId};

// TODO: per-thread log

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
    CRITICAL,
}

pub const LEVELS: [LogLevel; 5] = [
    LogLevel::DEBUG,
    LogLevel::INFO,
    LogLevel::WARN,
    LogLevel::ERROR,
    LogLevel::CRITICAL
];

impl fmt::Display for LogLevel {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LogLevel::DEBUG => write!(f, "DEBUG"),
            &LogLevel::INFO => write!(f, "INFO"),
            &LogLevel::WARN => write!(f, "WARN"),
            &LogLevel::ERROR => write!(f, "ERROR"),
            &LogLevel::CRITICAL => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Clone)]
pub struct Record<'a> {
    pub level: LogLevel,
    pub module: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub ts: DateTime<UTC>,
    pub args: Arc<fmt::Arguments<'a>>,

    msg: Arc<RwLock<Option<Arc<Box<String>>>>>,
    formatted: Arc<RwLock<Option<Arc<Box<String>>>>>,
    formatter: &'a Formatter<'a>,
}

impl<'a> Record<'a> {
    #[inline(always)]
    fn new(level: LogLevel, module: &'static str, file: &'static str, line: u32,
           ts: DateTime<UTC>, args: fmt::Arguments<'a>,
           formatter: &'a Formatter<'a>) -> Self {
        Record {
            level: level,
            module: module,
            file: file,
            line: line,
            ts: ts,
            args: Arc::new(args),
            msg: Arc::new(RwLock::new(None)),
            formatted: Arc::new(RwLock::new(None)),
            formatter: formatter,
        }
    }

    pub fn msg(&self) -> Arc<Box<String>> {
        let msg = self.msg.clone();
        {
            let mut msg = msg.write().unwrap();
            if msg.is_none() {
                let mut mstr = String::new();
                mstr.write_fmt(*self.args.clone()).unwrap();

                *msg = Some(Arc::new(Box::new(mstr)));
            }
        }
        let msg = msg.read().unwrap();
        let msg = msg.as_ref().unwrap();
        msg.clone()
    }

    pub fn formatted(&self) -> Arc<Box<String>> {
        let formatted = self.formatted.clone();
        {
            let mut formatted = formatted.write().unwrap();
            if formatted.is_none() {
                *formatted = Some(Arc::new((self.formatter)(self)));
            }
        }
        let formatted = formatted.read().unwrap();
        let formatted = formatted.as_ref().unwrap();
        formatted.clone()
    }
}

pub type Formatter<'a> = Box<Fn(&Record) -> Box<String> + 'a>;
pub type Handler<'a> = Box<Fn(&Record) + 'a>;

#[derive(Debug)]
struct LPath<'a> {
    path: Vec<&'a str>,
}

#[derive(Debug)]
struct LPathIter<'a> {
    path: &'a LPath<'a>,
    index: usize,
}

impl<'a> LPath<'a> {
    fn new(path: &'a String) -> Self {
        let mut lpath = Vec::new();

        for path in path.split("::") {
            for path in path.split(path::MAIN_SEPARATOR) {
                if !path.is_empty() {
                    lpath.push(path);
                }
            }
        }

        LPath {
            path: lpath,
        }
    }

    fn iter(&self) -> LPathIter {
        LPathIter {
            path: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for LPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.index >= self.path.path.len() {
            None
        } else {
            let index = self.index;
            self.index += 1;
            Some(self.path.path[index])
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    pub level: LogLevel,
    sub: HashMap<String, Arc<Logger>>,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            level: LogLevel::WARN,
            sub: HashMap::new(),
        }
    }

    pub fn level(&mut self, level: LogLevel) {
        self.level = level;
        // XXX: or propagate the the level?
        self.sub.clear();
    }

    pub fn sublevel<'a, T>(&mut self, mut path: T, level: LogLevel) where T: Iterator<Item = &'a str> {
        match path.next() {
            None => self.level(level),
            Some(ref piece) => {
                let mut logger = self.sub.entry(piece.to_string()).or_insert(Arc::new(Logger::new()));
                let mut logger = Arc::get_mut(&mut logger).unwrap();
                logger.sublevel(path, level);
            },
        }
    }
}

pub struct RootLogger<'a> {
    root: Arc<Logger>,
    formatter: Formatter<'a>,
    handlers: LinkedList<Handler<'a>>,
}

impl<'a> RootLogger<'a> {
    #[allow(dead_code)]
    fn reset(&mut self) {
        self.root = Arc::new(Logger::new());
        self.formatter =  Box::new(::formatters::default::formatter);
        self.handlers = LinkedList::new();
    }

    fn get_logger_slow<'b>(&self, module: &'static str, file: &'static str)
                           -> Arc<Logger> {
        let mut logger = self.root.clone();

        for path in module.split("::") {
            logger = {
                let sub = &logger.sub;
                match sub.get(path) {
                    Some(logger) => logger.clone(),
                    None => { return logger.clone(); },
                }
            }
        }

        for path in file.split(path::MAIN_SEPARATOR) {
            logger = {
                let sub = &logger.sub;
                match sub.get(path) {
                    Some(logger) => logger.clone(),
                    None => { return logger.clone(); },
                }
            }
        }

        logger
    }

    #[inline]
    fn get_logger<'b>(&self, module: &'static str, file: &'static str)
                      -> Arc<Logger> {
        let logger = self.root.clone();
        if logger.sub.is_empty() {
            logger
        } else {
            self.get_logger_slow(module, file)
        }
    }
}

impl<'a> RootLogger<'a> {
    pub fn new() -> Self {
        RootLogger {
            root: Arc::new(Logger::new()),
            formatter: Box::new(::formatters::default::formatter),
            handlers: LinkedList::new(),
        }
    }

    #[inline]
    pub fn root(&self) -> Arc<Logger> {
        self.root.clone()
    }

    pub fn handler(&mut self, handler: Handler<'a>) {
        self.handlers.push_front(handler);
    }

    pub fn formatter(&mut self, formatter: Formatter<'a>) {
        self.formatter = formatter;
    }

    pub fn level(&mut self, path: &String, level: LogLevel) {
        let mut logger = Arc::make_mut(&mut self.root);
        logger.sublevel(LPath::new(path).iter(), level);
    }

    pub fn logger(&self, path: &String) -> Arc<Logger> {
        let mut logger = self.root.clone();

        for path in path.split("::") {
            for path in path.split(path::MAIN_SEPARATOR) {
                logger = {
                    let sub = &logger.sub;
                    match sub.get(path) {
                        Some(logger) => logger.clone(),
                        None => { return logger.clone(); },
                    }
                };
            }
        }

        logger
    }

    pub fn log(&self, level: LogLevel, module: &'static str, file: &'static str, line: u32, args: fmt::Arguments) {
        let logger = self.get_logger(module, file);

        if logger.level > level {
            return;
        }

        let record = Record::new(level, module, file, line, UTC::now(), args, &self.formatter);

        if self.handlers.is_empty() {
            ::handlers::stdout::emit(&record.formatted());
        } else {
            for h in self.handlers.iter() {
                (*h)(&record);
            }
        }
    }
}

#[derive(Clone)]
struct LoggersReader<'a> {
    inner: Arc<RwLock<RootLogger<'a>>>
}

fn loggers() -> LoggersReader<'static> {
    static mut LOGGERS: *const LoggersReader<'static> = 0 as *const LoggersReader;
    static ONCE: Once = ONCE_INIT;
    unsafe {
        ONCE.call_once(|| {
            let loggers = LoggersReader {
                inner: Arc::new(RwLock::new(RootLogger::new())),
            };
            LOGGERS = mem::transmute(Box::new(loggers));
        });

        (*LOGGERS).clone()
    }
}

pub fn root() -> Arc<RwLock<RootLogger<'static>>> {
    let loggers = loggers();
    loggers.inner.clone()
}

pub fn is_string<T: ?Sized + Any>(_: &T) -> bool {
    TypeId::of::<String>() == TypeId::of::<T>() || TypeId::of::<&str>() == TypeId::of::<T>()
}

pub fn is_logger_level<T: ?Sized + Any>(_: &T) -> bool {
    TypeId::of::<LogLevel>() == TypeId::of::<T>()
}

#[macro_export]
macro_rules! __action {
    ($logger:expr, $func:ident($arg:expr)) => {{
        let root = $crate::logger::root();
        let mut root = root.write().unwrap();
        root.$func(&$logger.to_string(), $arg);
    }};
    ($func:ident($arg:expr)) => {{
        let root = $crate::logger::root();
        let mut root = root.write().unwrap();
        root.$func(&"".to_string(), $arg);
    }};
}

#[macro_export]
macro_rules! level {
    // setters
    ($logger:expr, [ $level:expr ]) => {{
        if !$crate::logger::is_string(&$logger) {
            panic!("Logger name must be a string");
        }
        __action!(&$logger.to_string(), level($level));
    }};
    ([ $level:expr ]) => {{
        __action!(level($level));
    }};

    // getters
    () => {{
        let root = $crate::logger::root();
        let root = root.read().unwrap().root().clone();
        root.level
    }};
    ($logger:expr) => {{
        let root = $crate::logger::root();
        if !$crate::logger::is_string(&$logger) {
            if $crate::logger::is_logger_level(&$logger) {
                panic!(format!("You might have meant [LogLevel::{:?}]", $logger));
            }
            panic!("Logger name must be a string");
        }
        let logger = root.read().unwrap().logger(&$logger.to_string()).clone();
        logger.level
    }};
}

#[macro_export]
macro_rules! __waction {
    ($func:ident($arg:expr)) => {{
        let root = $crate::logger::root();
        root.write().unwrap().$func($arg);
    }};
}

#[macro_export]
macro_rules! handler {
    ($handler:expr) => {{
        __waction!(handler($handler));
    }};
}

#[macro_export]
macro_rules! formatter {
    ($formatter:expr) => {{
        __waction!(formatter($formatter));
    }};
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {{
        let root = $crate::logger::root();
        root.read().unwrap().log($level, module_path!(), file!(), line!(), format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::DEBUG, $($arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log!($crate::LogLevel::INFO, $($arg)*);
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
mod test {
    use super::*;

    use std::mem;
    use std::sync::{Arc, RwLock, Mutex, Once, ONCE_INIT};
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
            {
                let root = root();
                let mut root = root.write().unwrap();
                root.reset();
            }

            let out = Arc::new(Mutex::new(String::new()));
            {
                let out = out.clone();
                handler!(Box::new(move |record| {
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
            assert_eq!(level!(), LogLevel::WARN);
            assert_eq!(level!("foo"), LogLevel::WARN);

            level!([LogLevel::INFO]);
            assert_eq!(level!(), LogLevel::INFO);
            assert_eq!(level!("foo"), LogLevel::INFO);
        });
    }

    #[test]
    fn test_logger_basic() {
        run_test(|buf| {
            for l in LEVELS.iter() {
                level!([*l]);
                assert_eq!(*l, level!());

                for l in LEVELS.iter() {
                    log!(*l, "msg");
                    let mut output = buf.lock().unwrap();
                    if *l >= level!() {
                        assert!(output.as_str().contains("msg"));
                        assert!(output.as_str().contains(&l.to_string()));
                    } else {
                        assert!(output.is_empty());
                    }
                    output.clear();
                }
            }

            let logger = "woodpecker";
            level!([LogLevel::WARN]);
            level!(logger, [LogLevel::DEBUG]);
            assert_eq!(level!(), LogLevel::WARN);
            assert_eq!(level!("foo"), LogLevel::WARN);
            assert_eq!(level!(logger), LogLevel::DEBUG);
            {
                debug!("msg");
                let output = buf.lock().unwrap();
                assert!(output.as_str().contains("msg"));
                assert!(output.as_str().contains("DEBUG"));
            }
        });
    }

    #[test]
    fn test_logger_handler() {
        run_test(|buf| {
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                handler!(Box::new(move |record| {
                    out.write().unwrap().push_str(record.msg().deref());
                    out.write().unwrap().push_str("|");
                }));

                level!([LogLevel::INFO]);
                info!("msg");
                debug!("foo");
            }
            assert_eq!(buf.lock().unwrap().split("msg").count(), 2);
            assert_eq!(*out.read().unwrap(), "msg|".to_string());
        });
    }

    #[test]
    fn test_logger_formatter() {
        run_test(|_| {
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                handler!(Box::new(move |record| {
                    out.write().unwrap().push_str(record.formatted().deref());
                }));
                formatter!(Box::new(|record| {
                    Box::new(format!(
                        "{}:{}|",
                        record.level,
                        record.msg(),
                    ))
                }));

                level!([LogLevel::INFO]);
                info!("msg");
                debug!("foo");
            }
            assert_eq!(*out.read().unwrap(), "INFO:msg|".to_string());
        });
    }

    #[test]
    fn test_logger_threads() {
        run_test(|_| {
            let thqty = 100;
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                handler!(Box::new(move |record| {
                    out.write().unwrap().push_str(record.formatted().deref());
                }));
                formatter!(Box::new(move |record| {
                    Box::new(format!(
                        "{}:{}",
                        record.level,
                        record.msg(),
                    ))
                }));

                let mut threads = Vec::new();
                level!([LogLevel::INFO]);
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
            let sum = out.read().unwrap().split("INFO:").
                filter(|val| !val.is_empty()).
                fold(0, |acc, ref val| {
                    acc + val.parse::<u32>().unwrap()
                });
            let expected = (0..100).filter(|x| x % 2 == 0).sum();
            assert_eq!(sum, expected);
        });
    }
}
