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

#![warn(missing_docs)]

//! # Woodpecker - fast and extensible logger for Rust
//!
//! The main goal of `woodpecker` is to be incredibly fast, extensible and easy to use.
//!
//! Please check out the project [homepage](https://github.com/niamster/woodpecker)
//! for more details on features, issues and limitation.
//!
//! The log levels are treated in a hierarchical manner.
//!
//! If there's no exact match for a requested log then the closest match is used.
//!
//! That means that the log level for `foo::bar::qux` is deduced from the log level
//! for `foo::bar` unless the log level for `foo::bar::qux` is explicitly set.
//!
//! See documentation for the [wp_get_level](macro.wp_get_level.html) and [log](macro.log.html)
//! macros for more details on the hierarchy.
//!
//! # Installation
//! To start using `woodpecker` just enable it in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! woodpecker = "0.3"
//! ```
//!
//! In the very beginning `woodpecker` should be configured using [wp_init](macro.wp_init!.html) macro.
//! By default [WARN](levels/enum.LogLevel.html) is used.
//! The usual logging macros as `debug`, `info`, etc. are available
//! as with generic rust [logger](https://doc.rust-lang.org/log).
//!
//! # Example
//!
//! ```rust
//! #[macro_use]
//! extern crate woodpecker;
//! use woodpecker as wp;
//!
//! fn main() {
//!     wp_init!();
//!     wp_set_level!(wp::LogLevel::INFO).unwrap();
//!
//!     info!("{} is saying hello", "woodpecker");
//!     debug!("I'm invisible");
//! }
//!
//! ```
//!
//! Woodpecker supports logging in a dedicated thread.
//!
//! This frees the application from the latency caused by the overhead of log string
//! formatter and the log drain(terminal, file, network, etc.).
//!
//! It's important to use [sync](fn.sync.html) in the end of the `main` funtion
//! to ensure that all log records are properly flushed.
//!
//! The logging thread could be activated via a configuration parameter passed
//! to the [wp_init](macro.wp_init!.html) macro.
//! The `WP_LOG_THREAD` environment variable may be used overrides compile-time settings.

#[macro_use]
extern crate lazy_static;

#[doc(hidden)]
#[macro_use]
pub mod macros;

#[doc(hidden)]
pub mod config;
#[doc(inline)]
pub use config::Config;

/// Definition of the log levels.
pub mod levels;
#[doc(inline)]
pub use levels::LogLevel;

/// Definition of the log record entry.
pub mod record;
#[doc(inline)]
pub use record::Record;

/// The logger core.
#[macro_use]
pub mod logger;
#[doc(inline)]
pub use logger::{init, sync};

#[doc(hidden)]
pub mod line_range;
#[doc(inline)]
pub use line_range::LineRangeBound;
pub use line_range::LineRangeBound::{BOF, EOF};

/// Collection of log handlers.
pub mod handlers;

/// Collection of log record formatters.
pub mod formatters;

#[doc(hidden)]
pub mod global;

#[doc(inline)]
pub mod spec;
