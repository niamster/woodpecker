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

extern crate crossbeam;
use self::crossbeam::sync::SegQueue;
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
use config::Config;
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

type QVec = [SegQueue<AsyncRecord>; QNUM];

struct ModuleSpec {
    level: LogLevel,
    lranges: Vec<LineRangeSpec>,
}

#[doc(hidden)]
pub struct RootLogger {
    loggers: CachePadded<BTreeMap<String, ModuleSpec>>,
    formatter: Arc<Formatter>,
    handlers: Vec<Handler>,
    queue: Arc<QVec>,
}

impl RootLogger {
    fn new(queue: Arc<QVec>) -> Self {
        RootLogger {
            loggers: CachePadded::new(BTreeMap::new()),
            formatter: Arc::new(Box::new(::formatters::default::formatter)),
            handlers: Vec::new(),
            queue: queue,
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
        let eof: u32 = LineRangeBound::EOF.into();
        let range = self.loggers.range::<str, _>((Unbounded, Included(path)));
        for (name, logger) in range.rev() {
            if path.starts_with(name) {
                if line == eof || logger.lranges.is_empty() {
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
            self.queue[qidx].push(record);
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

fn lthread(root: Arc<RwLock<RootLogger>>, queues: Arc<QVec>) {
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
            for queue in queues.iter() {
                if let Some(record) = queue.try_pop() {
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
            let mut queues: QVec = mem::uninitialized();
            for queue in queues.iter_mut() {
                let z = mem::replace(queue, SegQueue::new());
                mem::forget(z);
            }
            let queues = Arc::new(queues);

            assert!(IS_INIT.load(Ordering::Relaxed));
            let root = Arc::new(RwLock::new(RootLogger::new(queues.clone())));

            if LOG_THREAD.load(Ordering::Relaxed) {
                let root = root.clone();
                thread::spawn(move || {
                    lthread(root, queues);
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

#[doc(hidden)]
pub fn init(config: Config) {
    let log_thread = match env::var("WP_LOG_THREAD") {
        Ok(ref val) => {
            let val = &val.to_lowercase()[..1];
            val == "y" || val == "1"
        }
        _ => config.thread,
    };
    LOG_THREAD.store(log_thread, Ordering::Relaxed);

    // NOTE: it's not a real guard.
    // The `init` function is supposed to be called once on init.
    assert!(!IS_INIT.swap(true, Ordering::Relaxed));

    reset();

    if let Ok(ref rust_log) = env::var("RUST_LOG") {
        wp_set_level!(spec(rust_log)).unwrap();
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
                let config = Config {
                    thread: cfg!(feature = "test-thread-log"),
                    ..Default::default()
                };
                wp_init!(config);

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
    fn test_logger_hierarchy_override() {
        run_test(|_| {
            wp_set_level!(LogLevel::CRITICAL, "foo::bar").unwrap();
            wp_set_level!(LogLevel::INFO, "foo").unwrap();

            assert_eq!(wp_get_level!(^), LogLevel::WARN);
            assert_eq!(wp_get_level!("foo::bar::qux::xyz"), LogLevel::INFO);
            assert_eq!(wp_get_level!("foo::bar::qux"), LogLevel::INFO);
            assert_eq!(wp_get_level!("foo::bar"), LogLevel::INFO);
            assert_eq!(wp_get_level!("foo"), LogLevel::INFO);
            assert_eq!(wp_get_level!("bar"), LogLevel::WARN);
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
            let expected: u32 = (0..100).filter(|x| x % 2 == 0).sum();
            assert_eq!(sum, expected);
        });
    }
}
