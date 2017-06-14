
extern crate clap;
extern crate libatomicio;

use std::io;

use clap::{App, Arg};
use libatomicio as atomic;

fn main() {
    let matches = App::new("atomicio")
        .version("0.1")
        .about("Modify files atomically")
        .author("Aaron Brodersen")
        .arg(Arg::with_name("FILE")
            .help("The file to be modified")
            .required(true)
            .index(1))
        .get_matches();

    let original = matches.value_of("FILE").unwrap();

    let mut file = atomic::AtomicFile::open(original);
    let mut input = io::stdin();

    io::copy(&mut input, &mut file).unwrap();

    file.commit();
}
