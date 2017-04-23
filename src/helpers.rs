// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::any::{Any, TypeId};

pub fn is_string<T: ?Sized + Any>(_: &T) -> bool {
    TypeId::of::<String>() == TypeId::of::<T>() || TypeId::of::<&str>() == TypeId::of::<T>()
}

#[macro_export]
macro_rules! __wp_logger_is_string {
    ($logger:expr) => {{
        if !$crate::helpers::is_string(&$logger) {
            panic!("Logger name must be a string");
        }
    }};
}

#[macro_export]
macro_rules! this_file {
    () => (concat!(module_path!(), wp_separator!(), file!()))
}

#[macro_export]
macro_rules! this_module {
    () => (module_path!())
}
