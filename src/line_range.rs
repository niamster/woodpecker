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

// XXX: shall it be replaced with `std::ops::Range`?
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Range {
    pub from: u32,
    pub to: u32,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum RangeError {
    InvalidRange(u32, u32),
}

impl Range {
    #[inline(always)]
    pub fn new(from: u32, to: u32) -> Result<Self, RangeError> {
        if from > to {
            Err(RangeError::InvalidRange(from, to))
        } else {
            Ok(Range {
                from: from,
                to: to,
            })
        }
    }

    pub fn contains_point(&self, point: u32) -> bool {
        point >= self.from && point <= self.to
    }

    pub fn intersects(&self, other: &Range) -> bool {
        self.contains_point(other.from)
            || self.contains_point(other.to)
            || other.contains_point(self.from)
    }
}

impl From<(u32, u32)> for Range {
    fn from(orig: (u32, u32)) -> Range {
        Range::new(orig.0, orig.1).unwrap()
    }
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct LineRangeSpec {
    pub level: LogLevel,
    pub range: Range,
}

impl LineRangeSpec {
    #[inline(always)]
    fn new(level: LogLevel, range: Range) -> Self {
        LineRangeSpec {
            level: level,
            range: range,
        }
    }

    pub fn contains(&self, line: u32) -> bool {
        self.range.contains_point(line)
    }
}

pub fn from_vtuple(lranges: &[(u32, u32)]) -> Result<Vec<Range>, RangeError> {
    let (lranges, fails): (Vec<_>, Vec<_>) = lranges.iter()
        .map(|&(from, to)| {
            Range::new(from, to)
        })
        .partition(Result::is_ok);
    if !fails.is_empty() {
        let err = *fails.first().unwrap();
        return Err(err.unwrap_err());
    }
    Ok(lranges.into_iter().map(Result::unwrap).collect())
}

pub(crate) fn merge_ranges(lranges: &[Range], squash: bool) -> Vec<Range> {
    if lranges.is_empty() {
        return Vec::new();
    }

    let mut sorted = Vec::new();
    sorted.extend_from_slice(lranges);

    sorted.sort_by_key(|range| range.from);

    let mut merged = Vec::new();
    let mut iter = sorted.into_iter();
    let mut prev = iter.next().unwrap();
    for item in iter {
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

    if squash && merged.len() == 1 {
        let bof: u32 = LineRangeBound::BOF.into();
        let eof: u32 = LineRangeBound::EOF.into();
        let first = *merged.first().unwrap();
        if first.from == bof && first.to == eof {
            merged.clear();
        }
    }

    merged
}

pub fn spec(level: LogLevel, lranges: &[Range]) -> Result<Vec<LineRangeSpec>, RangeError> {
    if lranges.is_empty() {
        return Ok(Vec::new());
    }

    Ok(merge_ranges(lranges, true).iter()
       .map(|range| LineRangeSpec::new(level, *range))
       .collect()
    )
}

fn __push(merged: &mut Vec<LineRangeSpec>, level: LogLevel, mut ranges: Vec<Range>) {
    if !merged.is_empty() && merged[merged.len() - 1].level == level {
        ranges.push(merged.pop().unwrap().range);
    }
    let ranges = merge_ranges(&ranges, false);
    for range in &ranges {
        merged.push(LineRangeSpec::new(level, *range));
    }
}

pub(crate) fn merge_spec(orig: &[LineRangeSpec], new: &[LineRangeSpec]) -> Vec<LineRangeSpec> {
    let (mut orig, mut new): (Vec<_>, Vec<_>) = (orig.into(), new.into());
    let (mut orig, mut new) = (orig.iter_mut(), new.iter_mut());
    let (mut oi, mut ni) = (orig.next(), new.next());
    let mut merged = Vec::new();

    loop {
        if oi.is_none() && ni.is_none() {
            break;
        }
        if oi.is_none() && ni.is_some() {
            let niv = *ni.unwrap();
            __push(&mut merged, niv.level, vec![niv.range]);
            ni = new.next();
            continue;
        }
        if ni.is_none() && oi.is_some() {
            let oiv = *oi.unwrap();
            __push(&mut merged, oiv.level, vec![oiv.range]);
            oi = orig.next();
            continue;
        }

        let (oil, nil) = (oi.as_ref().unwrap().level, ni.as_ref().unwrap().level);
        if oil == nil {
            let (oiv, niv) = (oi.unwrap(), ni.unwrap());
            __push(&mut merged, oil, vec![oiv.range, niv.range]);
            oi = orig.next();
            ni = new.next();
        } else {
            let (ref oir, ref nir) = (oi.as_ref().unwrap().range, ni.as_ref().unwrap().range);
            if oir.intersects(nir) {
                if nir.from <= oir.from && nir.to >= oir.to {
                    oi = orig.next();
                } else if nir.from >= oir.from && nir.to <= oir.to {
                    if nir.from > oir.from {
                        __push(&mut merged, oil, vec![Range::new(oir.from, nir.from - 1).unwrap()]);
                    }
                    __push(&mut merged, nil, vec![*nir]);
                    if nir.to < oir.to {
                        oi.as_mut().unwrap().range.from = nir.to + 1;
                    } else {
                        oi = orig.next();
                    }
                    ni = new.next();
                } else if nir.from <= oir.from {
                    __push(&mut merged, nil, vec![*nir]);
                    oi.as_mut().unwrap().range.from = nir.to + 1;
                    ni = new.next();
                } else if nir.to >= oir.to {
                    __push(&mut merged, oil, vec![Range::new(oir.from, nir.from - 1).unwrap()]);
                    oi = orig.next();
                } else {
                    panic!("Unhandled case for ({:?}, {:?})", oi.unwrap(), ni.unwrap());
                }
            } else if oir.from < nir.from {
                __push(&mut merged, oil, vec![*oir]);
                oi = orig.next();
            } else {
                __push(&mut merged, nil, vec![*nir]);
                ni = new.next();
            }
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_ranges_merge_spec_same_level() {
        let bof: u32 = LineRangeBound::BOF.into();
        let eof: u32 = LineRangeBound::EOF.into();
        let left = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 200).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(150, 350).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 350).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(150, 400).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(250, 260).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 200).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(250, 260).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(150, 250).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 250).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(250, 350).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 200).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(250, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(50, 450).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(50, 450).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 400).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(bof, 450).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(bof, 450).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(50, eof).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(50, eof).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(bof, eof).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(bof, eof).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));
    }

    #[test]
    fn test_line_ranges_merge_spec_diff_level() {
        let bof: u32 = LineRangeBound::BOF.into();
        let eof: u32 = LineRangeBound::EOF.into();
        let left = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 200).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 350).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 149).unwrap()),
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 350).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(351, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(100, 350).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(100, 350).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(351, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(100, 150).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(100, 150).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(151, 200).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 200).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 149).unwrap()),
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 200).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 400).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 149).unwrap()),
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(250, 260).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 200).unwrap()),
            LineRangeSpec::new(LogLevel::INFO, Range::new(250, 260).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 250).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 149).unwrap()),
            LineRangeSpec::new(LogLevel::INFO, Range::new(150, 250).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(300, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(250, 350).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(100, 200).unwrap()),
            LineRangeSpec::new(LogLevel::INFO, Range::new(250, 350).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(351, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(50, 450).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(50, 450).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(100, 400).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(100, 400).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(bof, 450).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(bof, 450).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(50, eof).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(50, eof).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));

        let right = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(bof, eof).unwrap()),
        ];
        let expect = vec![
            LineRangeSpec::new(LogLevel::INFO, Range::new(bof, eof).unwrap()),
        ];
        assert_eq!(expect, merge_spec(&left, &right));
    }

    #[test]
    fn test_line_ranges_intersection() {
        let range = Range::new(100, 200).unwrap();

        let tvec = vec![
            Range::new(90, 100).unwrap(),
            Range::new(90, 101).unwrap(),
            Range::new(199, 210).unwrap(),
            Range::new(200, 210).unwrap(),
            Range::new(110, 190).unwrap(),
            Range::new(90, 210).unwrap(),
        ];
        for t in &tvec {
            assert!(range.intersects(t));
            assert!(t.intersects(&range));
        }

        let tvec = vec![
            Range::new(90, 99).unwrap(),
            Range::new(201, 210).unwrap(),
        ];
        for t in &tvec {
            assert!(!range.intersects(t));
            assert!(!t.intersects(&range));
        }
    }

    #[test]
    fn test_line_ranges_merged() {
        // empty
        let output = merge_ranges(&Vec::new(), true);
        let expected = Vec::<Range>::new();
        assert_eq!(expected, output);

        // empty
        let output = spec(LogLevel::TRACE, &Vec::new()).unwrap();
        let expected = Vec::<LineRangeSpec>::new();
        assert_eq!(expected, output);

        // invalid range
        let output = from_vtuple(&vec![(15, 30), (10, 20), (20, 10)]);
        let expected = Err(RangeError::InvalidRange(20, 10));
        assert_eq!(expected, output);

        // proper merge
        let input = vec![(15, 30).into(), (10, 20).into()];
        let output = spec(LogLevel::TRACE, &input).unwrap();
        let expected = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(10, 30).unwrap()),
        ];
        assert_eq!(expected, output);

        // proper merge
        let input = vec![(15, 30).into(), (10, 20).into(), (40, 50).into()];
        let output = spec(LogLevel::TRACE, &input).unwrap();
        let expected = vec![
            LineRangeSpec::new(LogLevel::TRACE, Range::new(10, 30).unwrap()),
            LineRangeSpec::new(LogLevel::TRACE, Range::new(40, 50).unwrap()),
        ];
        assert_eq!(expected, output);

        // proper merge [special case for RootLogger::set_level]
        let input = vec![
            (LineRangeBound::BOF.into(), 30).into(),
            (10, 20).into(),
            (30, LineRangeBound::EOF.into()).into(),
        ];
        let output = spec(LogLevel::TRACE, &input).unwrap();
        let expected = Vec::<LineRangeSpec>::new();
        assert_eq!(expected, output);
    }
}
