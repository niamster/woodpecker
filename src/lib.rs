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
//! The main goal of `woodpecker` is to be incredibly fast for the code
//! that doesn't generate the log.
//!
//! Please check out the project [homepage](https://github.com/niamster/woodpecker)
//! for more details on features, issues and limitation.
//!
//! The log levels are assigned in a hierarchical manner.
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
//! To start using `woodpecker` it's enough to just enable it in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! woodpecker = "0.1"
//! ```
//!
//! In your `main.rs`:
//!
//! ```rust
//! #[macro_use]
//! extern crate woodpecker;
//! use woodpecker as wp;
//!
//! fn main() {
//!     wp::init();
//!
//!     warn!("It's alive!");
//! }
//! ```
//!
//! And start using logging functions as `debug`, `info`, etc. as with
//! generic rust [logger](https://doc.rust-lang.org/log)
//!
//! # Example
//!
//! ```rust
//! #[macro_use]
//! extern crate woodpecker;
//! use woodpecker as wp;
//!
//! fn main() {
//!     wp::init();
//!
//!     wp_set_level!(wp::LogLevel::INFO);
//!     info!("{} is saying hello", "woodpecker");
//!     debug!("I'm invisible");
//! }
//!
//! ```
//!
//! Woodpecker supports logging in a dedicated thread.
//!
//! This frees the application from the latency caused by formatting
//! the log string and emitting it into a log sink(terminal, file, network, etc.).
//!
//! It's important to use [sync](fn.sync.html) in the end of the `main` funtion
//! to ensure that all log records are properly flushed.
//!
//! The logging thread is either activated explicitly via
//! [init_with_thread](fn.init_with_thread.html) function or by setting
//! `WP_LOG_THREAD` environment variable which overrides compile-time settings.

#[doc(hidden)]
#[macro_use]
pub mod helpers;

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
pub use logger::{init, init_with_thread, sync};

/// Collection of log handlers.
pub mod handlers;

/// Collection of log record formatters.
pub mod formatters;
