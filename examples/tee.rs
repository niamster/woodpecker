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
    level!([wp::LogLevel::INFO]);
    handler!(wp::handlers::stdout::handler());
    formatter!(Box::new(|record| {
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
        handler!(wp::handlers::file::handler(path));
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
