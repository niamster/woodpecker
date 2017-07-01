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

/// Initializes the crate's kitchen.
///
/// An optional parameter may be passed to define initial
/// logging [configuration](struct.Config.html).
///
/// Takes into account `RUST_LOG` environment variable
/// which must conform to the [spec](spec/index.html).
///
/// The `WP_LOG_THREAD` environment variable overrides the passed configuration.
///
/// # Example
///
/// ```rust
/// #[macro_use]
/// extern crate woodpecker;
/// use woodpecker as wp;
///
/// fn main() {
///     wp_init!();
///
///     wp_set_level!(wp::LogLevel::INFO).unwrap();
///     info!("Coucou!");
/// }
///
/// ```
///
/// If log thread is activated consider using [sync](fn.sync.html)
/// to ensure that all log records are properly flushed.
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
///     wp_init!(&wp::Config { thread: true, ..Default::default() });
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
#[macro_export]
macro_rules! wp_init {
    () => {{
        let config: $crate::Config = Default::default();
        wp_init!(&config)
    }};
    ($config:expr) => {{
        $crate::init($config)
    }};
}
