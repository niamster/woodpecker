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

use std;

use levels::LogLevel;

/// The markers of the begging of the file and its end.
pub enum LineRangeBound {
    /// Beginning of the file.
    BOF,
    /// End of the file.
    EOF,
}

impl From<LineRangeBound> for u32 {
    #[inline(always)]
    fn from(orig: LineRangeBound) -> u32 {
        match orig {
            LineRangeBound::BOF => 0,
            LineRangeBound::EOF => std::u32::MAX,
        }
    }
}

#[doc(hidden)]
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct LineRangeSpec {
    pub level: LogLevel,
    pub from: u32,
    pub to: u32,
}

#[doc(hidden)]
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum LineRangeError {
    InvalidRange(u32, u32),
}

impl LineRangeSpec {
    #[doc(hidden)]
    #[inline(always)]
    pub fn new(level: LogLevel, from: u32, to: u32) -> Result<Self, LineRangeError> {
        if from > to {
            Err(LineRangeError::InvalidRange(from, to))
        } else {
            Ok(LineRangeSpec {
                level: level,
                from: from,
                to: to,
            })
        }
    }
}

impl ToString for LineRangeError {
    fn to_string(&self) -> String {
        match *self {
            LineRangeError::InvalidRange(from, to) => format!("Invalid range [{}; {}]", from, to),
        }
    }
}

#[doc(hidden)]
pub fn prepare_ranges(level: LogLevel, lranges: &[(u32, u32)]) -> Result<Vec<LineRangeSpec>, LineRangeError> {
    let (lranges, fails): (Vec<_>, Vec<_>) = lranges.iter()
        .map(|&(from, to)| LineRangeSpec::new(level, from, to))
        .partition(Result::is_ok);
    if !fails.is_empty() {
        let err = fails.first().unwrap().clone();
        return Err(err.unwrap_err());
    }

    let mut lranges: Vec<_> = lranges.into_iter().map(Result::unwrap).collect();
    lranges.sort_by_key(|k| k.from);

    // FIXME: merge the ranges

    if lranges.len() == 1 {
        let bof: u32 = LineRangeBound::BOF.into();
        let eof: u32 = LineRangeBound::EOF.into();
        let first = lranges.first().unwrap().clone();
        if first.from == bof && first.to == eof {
            lranges.clear();
        }
    }

    Ok(lranges)
}
