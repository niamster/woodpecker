# ![Woodpecker](https://cdn.rawgit.com/niamster/woodpecker/master/logo/woodpecker.png)

[![Crates.io](https://img.shields.io/crates/v/woodpecker.svg)](https://crates.io/crates/woodpecker)
[![Docs.rs](https://docs.rs/woodpecker/badge.svg)](https://docs.rs/woodpecker)
[![Linux Build Status](https://travis-ci.org/niamster/woodpecker.svg?branch=master)](https://travis-ci.org/niamster/woodpecker)
[![Windows Build Status](https://ci.appveyor.com/api/projects/status/ljd6pkh8fsx7oh7a/branch/master?svg=true)](https://ci.appveyor.com/project/niamster/woodpecker)
[![Codecov](https://codecov.io/gh/niamster/woodpecker/branch/master/graph/badge.svg)](https://codecov.io/gh/niamster/woodpecker)
[![Coveralls](https://coveralls.io/repos/github/niamster/woodpecker/badge.svg?branch=master)](https://coveralls.io/github/niamster/woodpecker?branch=master)
[![License](https://img.shields.io/crates/l/woodpecker.svg)](https://opensource.org/licenses/Apache-2.0)

# Woodpecker - Logging for [Rust][rust]

### Table of Contents

* [Status](#status)
* [`woodpecker` crate in your project](#in-your-project)
* [Features](#features)
* [License](#license)
* [Credits](#credits)

### Introduction

`woodpecker` is a logging framework for [Rust][rust].

The goal is to have a fast, extensible and easy logging in [Rust][rust] application.

[rust]: http://rust-lang.org

### Status

The project is currently under development and doesn't provide a lot of features.

Although the basic feature **logging** is well supported!

### Features
The main feature is almost zero overhead if no filtering rules are defined and log is not produced.

Currently supported:
* pluggable format function
* multiple log consumers
* filtering by module (any part of the module path)
* filtering by file (any part of the file path)
* logging to stdout/stderr
* logging to a file
* log file rotation (by size)

### Documentation

Most of the useful documentation can be gotten using rustdoc.

Check it out on [docs.rs/woodpecker](https://docs.rs/woodpecker).

### In your project

In Cargo.toml:

```
[dependencies]
woodpecker = "0.1"
```

In your `main.rs`:

```
#[macro_use]
extern crate woodpecker;
```

See [examples/basic.rs](https://github.com/niamster/woodpecker/blob/master/examples/basic.rs) for the quick overview.

### License
Woodpecker project is licensed under Apache-2.0 license.

Logo is licensed under Creative Commons Attribution (CC BY).

### Credits
Sprockets for logo are provided by Jon Daiello and Ray Uribe from the Noun Project under Creative Commons Attribution (CC BY).
