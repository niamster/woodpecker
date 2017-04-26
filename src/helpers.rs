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

use std::any::{Any, TypeId};

#[doc(hidden)]
pub fn is_string<T: ?Sized + Any>(_: &T) -> bool {
    TypeId::of::<String>() == TypeId::of::<T>() || TypeId::of::<&str>() == TypeId::of::<T>()
}

#[doc(hidden)]
#[macro_export]
macro_rules! __wp_logger_is_string {
    ($logger:expr) => {{
        if !$crate::helpers::is_string(&$logger) {
            panic!("Logger name must be a string");
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! this_file {
    () => (concat!(module_path!(), wp_separator!(), file!()))
}

#[doc(hidden)]
#[macro_export]
macro_rules! this_module {
    () => (module_path!())
}
