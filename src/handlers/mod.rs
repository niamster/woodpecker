// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use record::Record;

pub type Handler<'a> = Box<Fn(&Record) + 'a>;

pub mod stdout;
pub mod stderr;
pub mod file;
pub mod rotating_file;
