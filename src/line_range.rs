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
    if lranges.is_empty() {
        return Ok(Vec::new());
    }

    let (lranges, fails): (Vec<_>, Vec<_>) = lranges.iter()
        .map(|&(from, to)| LineRangeSpec::new(level, from, to))
        .partition(Result::is_ok);
    if !fails.is_empty() {
        let err = *fails.first().unwrap();
        return Err(err.unwrap_err());
    }

    let mut lranges: Vec<_> = lranges.into_iter().map(Result::unwrap).collect();
    lranges.sort_by_key(|k| k.from);

    let mut merged = Vec::new();
    let mut iter = lranges.into_iter();
    let mut prev = iter.next().unwrap();
    while let Some(item) = iter.next() {
        if prev.to >= item.from {
            if prev.to < item.to {
                prev.to = item.to;
            }
        } else {
            merged.push(prev);
            prev = item;
        }
    }
    merged.push(prev);

    if merged.len() == 1 {
        let bof: u32 = LineRangeBound::BOF.into();
        let eof: u32 = LineRangeBound::EOF.into();
        let first = *merged.first().unwrap();
        if first.from == bof && first.to == eof {
            merged.clear();
        }
    }

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_ranges_merged() {
        // empty
        let output = prepare_ranges(LogLevel::TRACE, &Vec::new()).unwrap();
        let expected = Vec::<LineRangeSpec>::new();
        assert_eq!(expected, output);

        // invalid range
        let input = vec![(15, 30), (10, 20), (20, 10)];
        let output = prepare_ranges(LogLevel::TRACE, &input);
        let expected = Err(LineRangeError::InvalidRange(20, 10));
        assert_eq!(expected, output);

        // proper merge
        let input = vec![(15, 30), (10, 20)];
        let output = prepare_ranges(LogLevel::TRACE, &input).unwrap();
        let expected = vec![
            LineRangeSpec::new(LogLevel::TRACE, 10, 30).unwrap()
        ];
        assert_eq!(expected, output);

        // proper merge
        let input = vec![(15, 30), (10, 20), (40, 50)];
        let output = prepare_ranges(LogLevel::TRACE, &input).unwrap();
        let expected = vec![
            LineRangeSpec::new(LogLevel::TRACE, 10, 30).unwrap(),
            LineRangeSpec::new(LogLevel::TRACE, 40, 50).unwrap(),
        ];
        assert_eq!(expected, output);

        // proper merge [special case for RootLogger::set_level]
        let input = vec![(LineRangeBound::BOF.into(), 30), (10, 20), (30, LineRangeBound::EOF.into())];
        let output = prepare_ranges(LogLevel::TRACE, &input).unwrap();
        let expected = Vec::<LineRangeSpec>::new();
        assert_eq!(expected, output);
    }
}
