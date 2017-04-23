// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[macro_use]
extern crate woodpecker;
use woodpecker as wp;

use std::fs::remove_file;
use std::path::Path;
use std::env;
use std::process;
use std::ops::Deref;

fn usage() -> ! {
    println!("tee [-a] [FILES...]");
    process::exit(1);
}

fn main() {
    wp_set_level!(wp::LogLevel::INFO);
    wp_set_handler!(wp::handlers::stdout::handler());
    wp_set_formatter!(Box::new(|record| {
        record.msg().deref().clone()
    }));

    let args: Vec<_> = env::args().collect();
    let mut files = Vec::new();
    let mut truncate = true;
    for arg in &args[1..] {
        if &arg[0..1] == "-" {
            if arg == "-a" {
                truncate = false;
            } else {
                usage();
            }
        } else {
            files.push(arg);
        }
    }

    for path in &files {
        let path = Path::new(path);
        if truncate {
            let _ = remove_file(path);
        }
        wp_set_handler!(wp::handlers::file::handler(path));
    }

    let stdin = std::io::stdin();
    loop {
        let mut buffer = String::new();
        match stdin.read_line(&mut buffer) {
            Ok(count) if count > 0 => { info!("{}", buffer); },
            _ => { break; },
        }
    }
}
