extern crate time;
extern crate getopts;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate num;

mod cpu;
mod mem_map;
mod read_bytes;
mod write_bytes;

use std::env;
use std::fs::File;
use std::io::Read;

fn usage(opts: &getopts::Options) {
    let prog = env::args().next().unwrap();
    println!("{}", opts.usage(&format!("usage: {} [options] <rom>", prog)));
}

fn main() {
    env_logger::init().unwrap();
    let mut opts = getopts::Options::new();

    opts
        .optflag("h", "help", "show this message");

    let matches = match opts.parse(env::args().skip(1)) {
        Ok(m) => m,
        Err(f) => panic!("{}", f),
    };

    if matches.opt_present("h") || matches.opt_present("help") || matches.free.len() == 0 {
        return usage(&opts);
    }

    let mut rom = vec!();

    let file = File::open(&matches.free[0]);
    match file.and_then(|mut f| f.read_to_end(&mut rom)) {
        Ok(..) => {},
        Err(e) => println!("failed to read {}: {}", matches.free[0], e),
    }
}
