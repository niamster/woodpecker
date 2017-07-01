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

//! The logger spec might also be either [env_logger](https://doc.rust-lang.org/log/env_logger) spec or
//! a JSON string which defines global and per-module log level.
//!
//! # Simple env_logger spec
//!
//! The [env_logger](https://doc.rust-lang.org/log/env_logger/#enabling-logging) spec defines
//! the way to set global logging level or on per module basis.
//!
//! The spec string has a form of:
//!
//! ```ignore
//! [level],[module=[level]],...
//! ```
//!
//! The level must be a valid logging [level](levels/enum.LogLevel.html) level in the range
//! from [TRACE](levels/enum.LogLevel.html) to [CRITICAL](levels/enum.LogLevel.html).
//!
//! If `level` doesn't match any known logging level it's treated as a module path.
//!
//! In this case if `level` is not specified the log level is set to [TRACE](levels/enum.LogLevel.html).
//!
//! # Extended JSON spec
//!
//! The JSON logging spec allows to specify a fine grained logging settings.
//!
//! It doesn't suffer from the issue when module or crate path match the logging level.
//!
//! Also it allows to define logging level for the ranges of lines and potentially define other
//! features supported by `woodpecker`.
//!
//! The definition of the JSON spec:
//!
//! ```json
//! {
//!     "level": "<global logging level>",
//!     "modules": [
//!         {
//!             "path": "<path to the module>",
//!             "level": "<module logging level>"
//!         },
//!         {
//!             "path": "<path to the module>",
//!             "lines": [[<from>, <to>], ...]
//!         },
//!         ...
//!     ]
//! }
//! ```
//!
//! If the global level is not specified then it's left untouched.
//!
//! The `path` field of the module object is mandatory while
//! the `level` and `lines` fields are optional.
//!
//! If module `level` is not specified then it defaults to [TRACE](levels/enum.LogLevel.html).
//!
//! In case the ranges of lines is omitted the logging for the whole file is defined.
//!
//! See documentation for the [wp_set_level](../macro.wp_set_level.html)
//! for examples.


extern crate serde_json;
use self::serde_json::Value;

use std;
use std::cmp::Ordering;

use levels::LogLevel;
use line_range;
use line_range::Range;

#[doc(hidden)]
#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub struct Module {
    pub path: String,
    pub level: LogLevel,
    pub lranges: Vec<Range>,
}

impl Module {
    fn with_level(path: &str, level: LogLevel) -> Self {
        Module {
            path: path.to_string(),
            level: level,
            lranges: Vec::new(),
        }
    }

    fn with_lranges(path: &str, level: LogLevel, lranges: Vec<Range>) -> Self {
        Module {
            path: path.to_string(),
            level: level,
            lranges: lranges,
        }
    }
}

#[doc(hidden)]
#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub struct Root {
    pub level: Option<LogLevel>,
    pub modules: Vec<Module>,
}

impl Root {
    fn new() -> Self {
        Root {
            level: None,
            modules: Vec::new(),
        }
    }

    #[cfg(test)]
    fn with_level(level: LogLevel) -> Self {
        Root {
            level: Some(level),
            modules: Vec::new(),
        }
    }

    #[cfg(test)]
    fn module(mut self, module: Module) -> Self {
        self.modules.push(module);
        squash(self).unwrap()
    }
}

/// JSON log spec parse failure.
#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub enum JsonError {
    /// Invalid JSON string.
    Json,
    /// The root is invalid.
    Root,
    /// The array of modules is invalid.
    Module,
    /// The log level of the root is invalid.
    RootLogLevel,
    /// The log level of the module is invalid.
    ModuleLogLevel,
    /// The line range of the module is invalid.
    LineRange,
    /// Line level intersection with different log levels.
    Intersection,
}

/// Generic log spec parse failure.
#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub enum ParseError {
    /// Invalid spec.
    Spec,
    /// Invalid log level.
    LogLevel,
    /// JSON parse error.
    Json(JsonError),
}

fn parse_json(json: &str) -> Result<Root, ParseError> {
    let spec: Value = serde_json::from_str(json)
        .or(Err(ParseError::Json(JsonError::Json)))?;
    let spec = spec.as_object()
        .ok_or(ParseError::Json(JsonError::Root))?;
    if spec.is_empty() {
        return Err(ParseError::Json(JsonError::Root));
    }

    let mut root = Root::new();

    if let Some(level) = spec.get("level") {
        root.level = Some(
            level.as_str()
                .ok_or(ParseError::Json(JsonError::RootLogLevel))?.parse()
                .or(Err(ParseError::Json(JsonError::RootLogLevel)))?
        );
    }

    if let Some(modules) = spec.get("modules") {
        let modules = modules.as_array()
            .ok_or(ParseError::Json(JsonError::Module))?;
        if modules.is_empty() {
            return Err(ParseError::Json(JsonError::Module));
        }
        for module in modules {
            let module = module.as_object()
                .ok_or(ParseError::Json(JsonError::Module))?;
            let path = module.get("path")
                .ok_or(ParseError::Json(JsonError::Module))?.as_str()
                .ok_or(ParseError::Json(JsonError::Module))?;
            let level = if let Some(level) = module.get("level") {
                level.as_str()
                    .ok_or(ParseError::Json(JsonError::ModuleLogLevel))?.parse()
                    .or(Err(ParseError::Json(JsonError::ModuleLogLevel)))?
            } else {
                LogLevel::TRACE
            };

            let module = if let Some(lines) = module.get("lines") {
                let lines = lines.as_array()
                    .ok_or(ParseError::Json(JsonError::LineRange))?;
                let mut lranges = Vec::new();
                if lines.is_empty() {
                    return Err(ParseError::Json(JsonError::LineRange));
                }
                for line in lines {
                    let line = line.as_array()
                        .ok_or(ParseError::Json(JsonError::LineRange))?;
                    if line.len() != 2 {
                        return Err(ParseError::Json(JsonError::LineRange));
                    }
                    let from = line[0].as_u64()
                        .ok_or(ParseError::Json(JsonError::LineRange))?;
                    if from > std::u32::MAX as u64 {
                        return Err(ParseError::Json(JsonError::LineRange))
                    }
                    let to = line[1].as_u64()
                        .ok_or(ParseError::Json(JsonError::LineRange))?;
                    if to > std::u32::MAX as u64 {
                        return Err(ParseError::Json(JsonError::LineRange))
                    }
                    if from > to {
                        return Err(ParseError::Json(JsonError::LineRange))
                    }
                    lranges.push(
                        Range::new(from as u32, to as u32)
                            .or(Err(ParseError::Json(JsonError::LineRange)))?
                    );
                }
                Module::with_lranges(path, level, lranges)
            } else {
                Module::with_level(path, level)
            };
            root.modules.push(module);
        }
    }

    Ok(root)
}

fn parse_token(root: &mut Root, token: &str) -> Result<(), ParseError> {
    if token.is_empty() {
        return Err(ParseError::Spec);
    }

    let mut kv = token.split('=');
    let (k, v) = (kv.next(), kv.next());

    if kv.next().is_some() {
        return Err(ParseError::Spec);
    }

    let k = k.unwrap().trim();
    if k.is_empty() {
        return Err(ParseError::Spec);
    }

    if v.is_none() {
        // `k` is either global log level or path for which log level is `TRACE`
        if let Ok(level) = k.to_uppercase().parse() {
            root.level = Some(level);
        } else {
            root.modules.push(Module::with_level(k, LogLevel::TRACE));
        }
    } else {
        let v = v.unwrap().trim();
        if v.is_empty() {
            return Err(ParseError::Spec);
        }

        if let Ok(level) = v.to_uppercase().parse() {
            root.modules.push(Module::with_level(k, level));
        } else {
            return Err(ParseError::Spec);
        }
    }

    Ok(())
}

fn squash(mut root: Root) -> Result<Root, ParseError> {
    if root.modules.is_empty() {
        return Ok(root);
    }

    root.modules
        .sort_by(|a, b|
                 match a.path.cmp(&b.path) {
                     Ordering::Equal => a.level.cmp(&b.level),
                     order => order,
                 }
    );
    for module in &mut root.modules {
        module.lranges = line_range::merge_ranges(&module.lranges, true);
    }

    let mut squashed = Root::new();
    squashed.level = root.level;

    // Squash
    let mut modules = Vec::new();
    let mut iter = root.modules.into_iter();
    let mut prev = iter.next().unwrap();
    for item in iter {
        if prev.path == item.path && prev.level == item.level  {
            let mut lranges = Vec::new();
            lranges.extend_from_slice(&prev.lranges);
            lranges.extend_from_slice(&item.lranges);
            prev.lranges = line_range::merge_ranges(&lranges, true);
        } else {
            modules.push(prev.clone());
            prev = item;
        }
    }
    modules.push(prev.clone());
    squashed.modules = modules;

    // Make sure that modules without precised line ranges appear first
    squashed.modules
        .sort_by(|a, b|
                 match a.path.cmp(&b.path) {
                     Ordering::Equal => {
                         if a.lranges.is_empty() {
                             Ordering::Less
                         } else if b.lranges.is_empty() {
                             Ordering::Greater
                         } else {
                             a.level.cmp(&b.level)
                         }
                     },
                     order => order,
                 }
    );

    // Make sure there are not intersections
    {
        let mut iter = squashed.modules.iter();
        let mut prev = iter.next().unwrap();
        for item in iter {
            if prev.path == item.path
                && !prev.lranges.is_empty()
                && prev.level != item.level {
                    for prange in &prev.lranges {
                        for irange in &item.lranges {
                            if prange.intersects(irange) {
                                return Err(ParseError::Json(JsonError::Intersection));
                            }
                        }
                    }
                }
            prev = item;
        }
    }

    Ok(squashed)
}

#[doc(hidden)]
pub fn parse(spec: &str) -> Result<Root, ParseError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(ParseError::Spec);
    }

    if spec.starts_with('{') {
        return parse_json(spec).and_then(squash);
    }

    let mut root = Root::new();
    for token in spec.split(',') {
        parse_token(&mut root, token)?;
    }

    squash(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_invalid() {
        assert_eq!(Err(ParseError::Spec), parse(""));
        assert_eq!(Err(ParseError::Spec), parse(","));
        assert_eq!(Err(ParseError::Spec), parse("="));
        assert_eq!(Err(ParseError::Spec), parse("foo="));
        assert_eq!(Err(ParseError::Spec), parse("=error"));
        assert_eq!(Err(ParseError::Spec), parse("foo,"));
        assert_eq!(Err(ParseError::Spec), parse(",foo"));
        assert_eq!(Err(ParseError::Spec), parse("foo=bar"));
        assert_eq!("Spec", format!("{:?}", parse("foo=bar").err().unwrap()));
    }

    #[test]
    fn test_spec_global() {
        let expect = Root::with_level(LogLevel::ERROR);
        assert_eq!(Ok(expect), parse("error"));
    }

    #[test]
    fn test_spec_module() {
        let expect = Root::new()
            .module(Module::with_level("foo", LogLevel::ERROR));
        assert_eq!(Ok(expect), parse("foo=error"));

        let expect = Root::new()
            .module(Module::with_level("foo", LogLevel::TRACE));
        assert_eq!(Ok(expect), parse("foo"));
    }

    #[test]
    fn test_spec_combined() {
        let expect = Root::with_level(LogLevel::CRITICAL)
            .module(Module::with_level("foo", LogLevel::TRACE))
            .module(Module::with_level("qux", LogLevel::TRACE))
            .module(Module::with_level("bar", LogLevel::ERROR));
        assert_eq!(Ok(expect), parse("critical,foo,bar=error,qux"));
    }

    #[test]
    fn test_spec_json() {
        let expect = Root::with_level(LogLevel::CRITICAL)
            .module(Module::with_level("foo", LogLevel::TRACE))
            .module(Module::with_level("bar", LogLevel::TRACE))
            .module(Module::with_lranges("bar", LogLevel::ERROR,
                                         vec!((10, 100).into(), (120, 130).into())));
        assert_eq!(Ok(expect), parse(r#"{
                                            "level": "critical",
                                            "modules": [
                                                {
                                                    "path": "foo"
                                                },
                                                {
                                                    "path": "bar"
                                                },
                                                {
                                                    "path": "bar",
                                                    "level": "error",
                                                    "lines": [
                                                        [10, 100], [120, 130]
                                                    ]
                                                }
                                            ]
                                        }"#));
    }

    #[test]
    fn test_spec_json_invalid() {
        assert_eq!(Err(ParseError::Json(JsonError::Json)), parse("{"));

        assert_eq!(Err(ParseError::Json(JsonError::Root)),
                   parse(r#"{}"#));

        assert_eq!(Err(ParseError::Json(JsonError::RootLogLevel)),
                   parse(r#"{"level": "foo"}"#));

        assert_eq!(Err(ParseError::Json(JsonError::Module)),
                   parse(r#"{"modules": {}}"#));
        assert_eq!(Err(ParseError::Json(JsonError::Module)),
                   parse(r#"{"modules": []}"#));
        assert_eq!(Err(ParseError::Json(JsonError::Module)),
                   parse(r#"{"modules": [{"level": "critical"}]}"#));

        assert_eq!(Err(ParseError::Json(JsonError::ModuleLogLevel)),
                   parse(r#"{"modules": [{"path": "bar", "level": "foo"}]}"#));

        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": {}}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": []}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[1]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[1, 10, 20]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[1, 4294967296]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[4294967296, 4294967297]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[0, 0.5]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[0.5, 1]]}]}"#));
        assert_eq!(Err(ParseError::Json(JsonError::LineRange)),
                   parse(r#"{"modules": [{"path": "bar", "lines": [[20, 10]]}]}"#));
    }

    #[test]
    fn test_spec_json_squash() {
        let spec = parse(r#"{"modules": [
                                {
                                    "path": "bar",
                                     "lines": [
                                        [120, 130], [10, 50]
                                     ]
                                },
                                {
                                    "path": "bar",
                                    "level": "error"
                                },
                                {
                                    "path": "bar",
                                     "lines": [
                                        [80, 120], [50, 80]
                                     ]
                                },
                                {
                                    "path": "bar",
                                    "level": "critical",
                                     "lines": [
                                        [131, 140]
                                     ]
                                },
                                {
                                    "path": "bar",
                                    "level": "error"
                                }
                            ]}"#).unwrap();
        let modules = vec![
            Module::with_level("bar", LogLevel::ERROR),
            Module::with_lranges("bar", LogLevel::TRACE, vec![
                (10, 130).into(),
            ]),
            Module::with_lranges("bar", LogLevel::CRITICAL, vec![
                (131, 140).into(),
            ]),
        ];
        assert_eq!(spec.modules, modules);
    }

    #[test]
    fn test_spec_json_squash_intersection() {
        let spec = parse(r#"{"modules": [
                                {
                                    "path": "bar",
                                     "lines": [
                                        [50, 80]
                                     ]
                                },
                                {
                                    "path": "bar",
                                    "level": "info",
                                     "lines": [
                                        [60, 70]
                                     ]
                                }
                            ]}"#);
        assert_eq!(Err(ParseError::Json(JsonError::Intersection)), spec)
    }
}
