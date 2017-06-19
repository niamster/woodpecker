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
    wp_init!();
    wp_register_handler!(wp::handlers::stdout::handler());
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
        wp_register_handler!(wp::handlers::file::handler(path).unwrap());
    }

    let stdin = std::io::stdin();
    loop {
        let mut buffer = String::new();
        match stdin.read_line(&mut buffer) {
            Ok(count) if count > 0 => {
                log!("{}", buffer);
            },
            _ => {
                break;
            },
        }
    }
}
