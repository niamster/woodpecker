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
use self::parking_lot::{RwLock, Mutex};

extern crate time;

extern crate crossbeam;
use self::crossbeam::sync::chase_lev;
use self::crossbeam::mem::CachePadded;

extern crate thread_id;

use std::mem;
use std::sync::{Arc, Once, ONCE_INIT};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering,
                        ATOMIC_USIZE_INIT, ATOMIC_BOOL_INIT};
use std::collections::BTreeMap;
use std::collections::Bound::{Included, Excluded, Unbounded};
use std::time::{Duration, Instant};
use std::thread;
use std::fmt;
use std::env;

use levels::LogLevel;
use record::Record;
use record::imp::{SyncRecord, AsyncRecord, RecordMeta};
use line_range::{LineRangeBound, LineRangeSpec};
use formatters::Formatter;
use handlers::Handler;
use global;

const QNUM: usize = 64;

static LOG_THREAD: AtomicBool = ATOMIC_BOOL_INIT;
static IS_INIT: AtomicBool = ATOMIC_BOOL_INIT;
lazy_static! {
    static ref SENT: [CachePadded<AtomicUsize>; QNUM] = {
        let mut sent: [CachePadded<AtomicUsize>; QNUM] = unsafe { mem::uninitialized() };
        for sent in sent.iter_mut() {
            let z = mem::replace(sent, CachePadded::new(ATOMIC_USIZE_INIT));
            mem::forget(z);
        }
        sent
    };
    static ref RECEIVED: CachePadded<AtomicUsize> = CachePadded::new(ATOMIC_USIZE_INIT);
}

/// A file-module separator.
///
/// This separator is used to join module and file paths.
#[macro_export]
macro_rules! wp_separator {
    () => ('@')
}

type QProducers = [Mutex<chase_lev::Worker<AsyncRecord>>; QNUM];
type QConsumers = [chase_lev::Stealer<AsyncRecord>; QNUM];

struct ModuleSpec {
    level: LogLevel,
    lranges: Vec<LineRangeSpec>,
}

#[doc(hidden)]
pub struct RootLogger {
    loggers: CachePadded<BTreeMap<String, ModuleSpec>>,
    formatter: Arc<Formatter>,
    handlers: Vec<Handler>,
    producers: QProducers,
}

impl RootLogger {
    fn new(producers: QProducers) -> Self {
        RootLogger {
            loggers: CachePadded::new(BTreeMap::new()),
            formatter: Arc::new(Box::new(::formatters::default::formatter)),
            handlers: Vec::new(),
            producers: producers,
        }
    }

    fn reset(&mut self) {
        self.loggers.clear();
        self.formatter = Arc::new(Box::new(::formatters::default::formatter));
        self.handlers.clear();
    }

    #[doc(hidden)]
    pub fn reset_loggers(&mut self) {
        self.loggers.clear()
    }

    #[doc(hidden)]
    pub fn handler(&mut self, handler: Handler) {
        self.handlers.push(handler);
    }

    #[doc(hidden)]
    pub fn formatter(&mut self, formatter: Formatter) {
        self.formatter = Arc::new(formatter);
    }

    fn remove_children(&mut self, path: &str) {
        let mut trash = Vec::new();
        {
            let range = self.loggers.range::<str, _>((Excluded(path), Unbounded));
            for (name, _) in range {
                if !name.starts_with(path) {
                    break;
                }
                trash.push(name.to_string());
            }
        }
        for item in trash {
            self.loggers.remove(item.as_str());
        }
    }

    #[doc(hidden)]
    pub fn set_level(&mut self, level: LogLevel, path: &str, lranges: Vec<LineRangeSpec>) -> Result<(), String> {
        if level == LogLevel::UNSUPPORTED {
            return Err("Unsupported log level".to_string());
        }

        if !lranges.is_empty() {
            if !path.ends_with("<anon>") && !path.ends_with(".rs") {
                return Err("File path not specified".to_string());
            }
            if path.find(wp_separator!()).is_none() {
                return Err("Module not specified".to_string());
            }
        }

        let level = if lranges.is_empty() {
            level
        } else {
            self.get_level(path, LineRangeBound::EOF.into())
        };

        self.remove_children(path);

        let logger = ModuleSpec {
            level: level,
            lranges: lranges,
        };
        self.loggers.insert(path.to_string(), logger);

        global::set_loggers(true);

        Ok(())
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn get_level(&self, path: &str, line: u32) -> LogLevel {
        let range = self.loggers.range::<str, _>((Unbounded, Included(path)));
        for (name, logger) in range.rev() {
            if path.starts_with(name) {
                if line == LineRangeBound::EOF.into() || logger.lranges.is_empty() {
                    return logger.level;
                }
                for range in &logger.lranges {
                    if line >= range.from && line <= range.to {
                        return range.level;
                    }
                }
                return logger.level;
            }
        }

        global::get_level()
    }

    #[doc(hidden)]
    pub fn log(&self, record: &'static RecordMeta, args: fmt::Arguments) {
        let record = SyncRecord::new(record, time::get_time(), args, self.formatter.clone());
        if !LOG_THREAD.load(Ordering::Relaxed) {
            self.process(&record);
        } else {
            let record: AsyncRecord = record.into();
            let qidx = thread_id::get() % QNUM;
            assert!(qidx < QNUM);
            {
                let mut producer = self.producers[qidx].lock();
                producer.push(record);
            }
            SENT[qidx].fetch_add(1, Ordering::Relaxed);
        }
    }

    #[inline(always)]
    fn process(&self, record: &Record) {
        if self.handlers.is_empty() {
            ::handlers::stdout::emit(&record.formatted());
        } else {
            for h in &self.handlers {
                h(record);
            }
        }
    }
}

fn qempty() -> bool {
    let sent = SENT.iter().fold(0, |sum, sent| sum + sent.load(Ordering::Relaxed));
    let received = RECEIVED.load(Ordering::Relaxed);
    sent == received
}

fn lthread(root: Arc<RwLock<RootLogger>>, consumers: &QConsumers) {
    const BWAIT_MS: u64 = 10;
    #[cfg(not(test))] const RWAIT_MS: u64 = 500;
    #[cfg(test)] const RWAIT_MS: u64 = 10;

    loop {
        'wait: loop {
            let now = Instant::now();
            while qempty() {
                thread::yield_now();
                if now.elapsed() > Duration::from_millis(BWAIT_MS) {
                    thread::sleep(Duration::from_millis(RWAIT_MS));
                    continue 'wait;
                }
            }
            if true {
                // Workaround for clippy
                // https://github.com/Manishearth/rust-clippy/issues/1586
                break;
            }
        }

        loop {
            let mut received: usize = 0;
            for consumer in consumers.iter() {
                if let chase_lev::Steal::Data(record) = consumer.steal() {
                    {
                        let root = root.read();
                        root.process(&record);
                    }
                    received += 1;
                }
                thread::yield_now();
            }
            RECEIVED.fetch_add(received, Ordering::Relaxed);
            if received < QNUM {
                break;
            }
            thread::yield_now();
        }
    }
}

#[inline(always)]
fn root() -> Arc<RwLock<RootLogger>> {
    static mut ROOT: *const Arc<RwLock<RootLogger>> = 0 as *const Arc<RwLock<RootLogger>>;
    static ONCE: Once = ONCE_INIT;
    unsafe {
        ONCE.call_once(|| {
            let mut producers: QProducers = mem::uninitialized();
            let mut consumers: QConsumers = mem::uninitialized();
            for idx in 0..QNUM {
                let (worker, stealer) = chase_lev::deque();
                let z = mem::replace(&mut producers[idx], Mutex::new(worker));
                mem::forget(z);
                let z = mem::replace(&mut consumers[idx], stealer);
                mem::forget(z);
            }

            assert!(IS_INIT.load(Ordering::Relaxed));
            let root = Arc::new(RwLock::new(RootLogger::new(producers)));

            if LOG_THREAD.load(Ordering::Relaxed) {
                let root = root.clone();
                thread::spawn(move || {
                    lthread(root, &consumers);
                });
                { // warm up lazy statics
                    sync();
                }
            }

            ROOT = mem::transmute(Box::new(root));
        });

        (*ROOT).clone()
    }
}

lazy_static! {
    #[doc(hidden)]
    pub static ref ROOT: Arc<RwLock<RootLogger>> = root();
}

/// Ensures that the logging queue is completely consumed by the log thread.
///
/// Normally this should be called in the very end of the program execution
/// to ensure that all log records are properly flushed.
pub fn sync() {
    while !qempty() {
        thread::sleep(Duration::from_millis(10));
    }
}

#[doc(hidden)]
pub fn reset() {
    sync();
    let mut root = ROOT.write();
    global::set_level(LogLevel::WARN);
    global::set_loggers(false);
    root.reset();
}

fn __init(log_thread: bool) {
    let log_thread = match env::var("WP_LOG_THREAD") {
        Ok(ref val) => {
            let val = &val.to_lowercase()[..1];
            val == "y" || val == "1"
        }
        _ => log_thread,
    };
    LOG_THREAD.store(log_thread, Ordering::Relaxed);

    // NOTE: it's not a real guard.
    // The `init` function is supposed to be called once on init.
    assert!(!IS_INIT.swap(true, Ordering::Relaxed));

    reset();
}

/// Initializes the crate's kitchen.
///
/// # Example
///
/// ```rust
/// #[macro_use]
/// extern crate woodpecker;
/// use woodpecker as wp;
///
/// fn main() {
///     wp::init();
///
///     wp_set_level!(wp::LogLevel::INFO).unwrap();
///     info!("Coucou!");
/// }
///
/// ```
pub fn init() {
    __init(false);
}

/// Same as [init](fn.init.html) but enables the log thread.
///
/// Consider using [sync](fn.sync.html) to ensure that all log
/// records are properly flushed.
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
///     wp::init_with_thread();
///
///     let out = Arc::new(Mutex::new(String::new()));
///     {
///         let out = out.clone();
///         wp_register_handler!(Box::new(move |record| {
///             out.lock().unwrap().push_str(record.msg().deref());
///         }));
///
///         warn!("foo");
///     }
///
///     wp::sync();
///
///     assert_eq!(*out.lock().unwrap(), "foo".to_string());
/// }
///
/// ```
pub fn init_with_thread() {
    __init(true);
}

#[doc(hidden)]
#[macro_export]
macro_rules! __wp_write_root {
    ($func:ident($($arg:expr),*)) => {{
        $crate::logger::sync();
        let mut root = $crate::logger::ROOT.write();
        root.$func($($arg),*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __wp_read_root {
    ($func:ident($($arg:expr),*)) => {{
        let root = $crate::logger::ROOT.read();
        root.$func($($arg),*)
    }};
}

/// Sets the log level.
///
/// Sets global log level if called without the arguments or
/// according to the specified path otherwise.
///
/// Optionally the ranges of the lines within the file could
/// be given.
/// In this case the full path to the file must be provided.
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
///     wp::init();
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
///     let ranges = vec![(wp::BOF.into(), line!() + 2), (line!() + 4, wp::EOF.into())];
///     wp_set_level!(wp::LogLevel::WARN, this_file!(), ranges).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::WARN);
///     assert_eq!(wp_get_level!(), wp::LogLevel::CRITICAL);
///     assert_eq!(wp_get_level!(), wp::LogLevel::WARN);
///
///     wp_set_level!(wp::LogLevel::TRACE, this_file!(), [(wp::BOF, wp::EOF)]).unwrap();
///     assert_eq!(wp_get_level!(), wp::LogLevel::TRACE);
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
        wp_set_level!($level, $logger, lranges)
    }};

    ($level:expr, $logger:expr, $lranges:expr) => {{
        match $crate::line_range::prepare_ranges($level, &$lranges) {
            Ok(lranges) => {
                __wp_write_root!(set_level($level, $logger, lranges))
            },
            Err(err) => {
                let err: Result<(), String> = Err(err);
                err
            },
        }
    }};

    ($level:expr, $logger:expr) => {{
        __wp_write_root!(set_level($level, $logger, Vec::new()))
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
///     wp::init();
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
///     wp::init();
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
/// that will be used as a return value of [Record::formatted](record/struct.Record.html#method.formatted) function.
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
///     wp::init();
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
                if cfg!(feature = "test-thread-log") {
                    init_with_thread();
                } else {
                    init();
                }

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
                wp_register_handler!(Box::new(move |record| {
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
            assert_eq!(wp_get_level!(^), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo"), LogLevel::WARN);

            wp_set_level!(LogLevel::INFO).unwrap();
            assert_eq!(wp_get_level!(^), LogLevel::INFO);
            assert_eq!(wp_get_level!("foo"), LogLevel::INFO);
        });
    }

    #[test]
    fn test_logger_hierarchy() {
        run_test(|_| {
            wp_set_level!(LogLevel::CRITICAL, "foo::bar::qux").unwrap();

            assert_eq!(wp_get_level!(^), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo::bar::qux::xyz"), LogLevel::CRITICAL);
            assert_eq!(wp_get_level!("foo::bar::qux"), LogLevel::CRITICAL);
            assert_eq!(wp_get_level!("foo::bar"), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo"), LogLevel::WARN);
        });
    }

    #[test]
    #[should_panic(expected = "File path not specified")]
    fn test_set_level_range_0() {
        run_test(|_| {
            wp_set_level!(LogLevel::TRACE, this_module!(), [(LineRangeBound::BOF, 42u32)]).unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "Module not specified")]
    fn test_set_level_range_1() {
        run_test(|_| {
            wp_set_level!(LogLevel::TRACE, file!(), [(LineRangeBound::BOF, 42u32)]).unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "Invalid range")]
    fn test_set_level_range_2() {
        run_test(|_| {
            wp_set_level!(LogLevel::TRACE, this_file!(), [(42u32, 41u32)]).unwrap();
        });
    }

    #[test]
    fn test_set_level_range_3() {
        run_test(|_| {
            assert_eq!(wp_set_level!(LogLevel::TRACE, "foo", [(LineRangeBound::BOF, LineRangeBound::EOF)]), Ok(()));
            assert_eq!(wp_set_level!(LogLevel::TRACE, this_file!(), [(LineRangeBound::BOF, 42u32)]), Ok(()));
        });
    }

    #[test]
    fn test_logger_basic() {
        run_test(|buf| {
            for l in LEVELS.iter() {
                wp_set_level!(*l).unwrap();
                assert_eq!(*l, wp_get_level!(^));
                assert_eq!(*l, wp_get_level!());

                for l in LEVELS.iter() {
                    match *l {
                        LogLevel::TRACE => trace!("msg"),
                        LogLevel::DEBUG => debug!("msg"),
                        LogLevel::VERBOSE => verbose!("msg"),
                        LogLevel::INFO => info!("msg"),
                        LogLevel::NOTICE => notice!("msg"),
                        LogLevel::WARN => warn!("msg"),
                        LogLevel::ERROR => error!("msg"),
                        LogLevel::CRITICAL => critical!("msg"),
                        LogLevel::LOG | LogLevel::UNSUPPORTED => panic!(),
                    }
                    sync();
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

            for l in LEVELS.iter() {
                wp_set_level!(*l).unwrap();
                assert_eq!(*l, wp_get_level!());

                log!(">>{}<<", "unconditional");
                sync();
                let mut output = buf.lock().unwrap();
                assert!(output.as_str().contains(">>unconditional<<"));
                output.clear();
            }

            wp_set_formatter!(Box::new(|record| {
                format!(
                    "{}:{}",
                    record.level(),
                    record.msg(),
                )
            }));

            let logger = "woodpecker";
            wp_set_level!(LogLevel::WARN).unwrap();
            wp_set_level!(LogLevel::VERBOSE, logger).unwrap();
            assert_eq!(wp_get_level!(^), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo"), LogLevel::WARN);
            assert_eq!(wp_get_level!(logger), LogLevel::VERBOSE);
            assert_eq!(wp_get_level!(), LogLevel::VERBOSE);

            {
                let mut output = buf.lock().unwrap();
                output.clear();
                drop(output);
                verbose!("msg");
                debug!("msg");
                sync();
                let output = buf.lock().unwrap();
                assert_eq!(output.as_str(), "VERBOSE:msg");
            }

            wp_set_level!(LogLevel::CRITICAL).unwrap();
            assert_eq!(wp_get_level!(), LogLevel::CRITICAL);

            let logger = this_module!();
            wp_set_level!(LogLevel::ERROR, logger).unwrap();
            assert_eq!(wp_get_level!(^), LogLevel::CRITICAL);
            assert_eq!(wp_get_level!(logger), LogLevel::ERROR);
            assert_eq!(wp_get_level!(), LogLevel::ERROR);

            let logger = this_file!();
            wp_set_level!(LogLevel::NOTICE, logger).unwrap();
            assert_eq!(wp_get_level!(^), LogLevel::CRITICAL);
            assert_eq!(wp_get_level!(logger), LogLevel::NOTICE);
            assert_eq!(wp_get_level!(), LogLevel::NOTICE);

            {
                let mut output = buf.lock().unwrap();
                output.clear();
                drop(output);
                notice!("msg");
                verbose!("msg");
                sync();
                let output = buf.lock().unwrap();
                assert_eq!(output.as_str(), "NOTICE:msg");
            }
        });
    }

    #[test]
    fn test_logger_in_log() {
        run_test(|buf| {
            wp_set_level!(LogLevel::WARN).unwrap();
            wp_set_formatter!(Box::new(|record| {
                (*record.msg()).clone()
            }));

            in_debug!({
                log!("hidden");
            });
            in_warn!({
                log!("visible");
            });

            sync();
            let output = buf.lock().unwrap();
            assert_eq!(output.as_str(), "visible");
        });
    }

    #[test]
    fn test_logger_handler() {
        run_test(|buf| {
            let out = Arc::new(RwLock::new(String::new()));
            {
                let out = out.clone();
                wp_register_handler!(Box::new(move |record| {
                    out.write().push_str(record.msg().deref());
                    out.write().push_str("|");
                }));

                wp_set_level!(LogLevel::INFO).unwrap();
                info!("msg");
                debug!("foo");
            }
            sync();
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
                wp_register_handler!(Box::new(move |record| {
                    out.write().push_str(record.formatted().deref());
                }));
                wp_set_formatter!(Box::new(|record| {
                    format!(
                        "{}:{}|",
                        record.level(),
                        record.msg(),
                    )
                }));

                wp_set_level!(LogLevel::INFO).unwrap();
                info!("msg");
                debug!("foo");
            }
            sync();
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
                wp_register_handler!(Box::new(move |record| {
                    out.write().push_str(record.formatted().deref());
                }));
                wp_set_formatter!(Box::new(move |record| {
                    format!(
                        "{}:{}",
                        record.level(),
                        record.msg(),
                    )
                }));

                let mut threads = Vec::new();
                wp_set_level!(LogLevel::INFO).unwrap();
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
            sync();
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
