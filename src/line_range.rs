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
use std::fmt;

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
#[derive(Clone)]
pub struct LineRange {
    pub level: LogLevel,
    pub from: u32,
    pub to: u32,
}

impl LineRange {
    #[doc(hidden)]
    #[inline(always)]
    pub fn new(level: LogLevel, from: u32, to: u32) -> Result<Self, String> {
        if from > to {
            Err(format!("Invalid range [{}; {}]", from, to))
        } else {
            Ok(LineRange {
                level: level,
                from: from,
                to: to,
            })
        }
    }
}

impl fmt::Debug for LineRange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}; {}]", self.from, self.to)
    }
}

#[doc(hidden)]
pub fn prepare_ranges(level: LogLevel, lranges: &[(u32, u32)]) -> Result<Vec<LineRange>, String> {
    let (lranges, fails): (Vec<_>, Vec<_>) = lranges.iter()
        .map(|&(from, to)| LineRange::new(level, from, to))
        .partition(Result::is_ok);
    if !fails.is_empty() {
        let err = fails.first().unwrap().clone();
        return Err(err.unwrap_err());
    }

    let mut lranges: Vec<_> = lranges.into_iter().map(Result::unwrap).collect();
    lranges.sort_by_key(|k| k.from);

    // FIXME: merge the ranges

    if lranges.len() == 1 {
        let first = lranges.first().unwrap().clone();
        if first.from == LineRangeBound::BOF.into()
            && first.to == LineRangeBound::EOF.into() {
            lranges.clear();
        }
    }

    Ok(lranges)
}
