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

extern crate parking_lot;
use self::parking_lot::RwLock;

use std::sync::Arc;

use std::fmt;
use std::fmt::Write;

use formatters::Formatter;
use levels::LogLevel;

struct PRecord<'a> {
    msg: RwLock<Option<Arc<Box<String>>>>,
    formatted: RwLock<Option<Arc<Box<String>>>>,
    formatter: &'a Formatter<'a>,
    ts_utc: RwLock<Option<Arc<DateTime<UTC>>>>,
}

impl<'a> PRecord<'a> {
    #[inline(always)]
    fn new(formatter: &'a Formatter<'a>) -> Self {
        PRecord {
            msg: RwLock::new(None),
            formatted: RwLock::new(None),
            formatter: formatter,
            ts_utc: RwLock::new(None),
        }
    }

    pub fn msg(&self, record: &Record) -> Arc<Box<String>> {
        {
            let mut msg = self.msg.write();
            if msg.is_none() {
                let mut mstr = String::new();
                mstr.write_fmt(record.args).unwrap();

                *msg = Some(Arc::new(Box::new(mstr)));
            }
        }
        let msg = self.msg.read();
        let msg = msg.as_ref().unwrap();
        msg.clone()
    }

    pub fn formatted(&self, record: &Record) -> Arc<Box<String>> {
        {
            let mut formatted = self.formatted.write();
            if formatted.is_none() {
                *formatted = Some(Arc::new((self.formatter)(record)));
            }
        }
        let formatted = self.formatted.read();
        let formatted = formatted.as_ref().unwrap();
        formatted.clone()
    }

    pub fn ts_utc(&self, record: &Record) -> Arc<DateTime<UTC>> {
        {
            let mut ts_utc = self.ts_utc.write();
            if ts_utc.is_none() {
                let naive = chrono::NaiveDateTime::from_timestamp(record.ts.sec, record.ts.nsec as u32);
                *ts_utc = Some(Arc::new(chrono::DateTime::from_utc(naive, chrono::UTC)));
            }
        }
        let ts_utc = self.ts_utc.read();
        let ts_utc = ts_utc.as_ref().unwrap();
        ts_utc.clone()
    }
}

#[derive(Clone)]
pub struct Record<'a> {
    pub level: LogLevel,
    pub module: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub ts: time::Timespec,
    pub args: fmt::Arguments<'a>,

    precord: Arc<PRecord<'a>>,
}

impl<'a> Record<'a> {
    #[inline(always)]
    pub fn new(level: LogLevel, module: &'static str, file: &'static str, line: u32,
           ts: time::Timespec, args: fmt::Arguments<'a>,
           formatter: &'a Formatter<'a>) -> Self {
        Record {
            level: level,
            module: module,
            file: file,
            line: line,
            ts: ts,
            args: args,

            precord: Arc::new(PRecord::new(formatter)),
        }
    }

    pub fn msg(&self) -> Arc<Box<String>> {
        self.precord.msg(self)
    }

    pub fn formatted(&self) -> Arc<Box<String>> {
        self.precord.formatted(self)
    }

    pub fn ts_utc(&self) -> Arc<DateTime<UTC>> {
        self.precord.ts_utc(self)
    }
}
