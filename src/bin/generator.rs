use rand::distributions::{Distribution, Uniform};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "generator")]
/// Generate f64 numbers and store to file
struct Config {
    /// Output file
    #[structopt(short = "o", long, parse(from_os_str))]
    output: PathBuf,
    /// Numbers to be generated
    #[structopt(short = "c", long)]
    count: usize,
}

fn main() {
    let conf: Config = Config::from_args();
    let mut f = BufWriter::with_capacity(1024 * 10, File::create(conf.output).unwrap());
    assert_ne!(conf.count, 0);
    //let uniform = Uniform::new(std::f64::MIN, std::f64::MAX);
    let uniform = Uniform::new(-100_f64, 100_f64);

    let mut rng = rand::thread_rng();
    for _ in 0..conf.count {
        let random_number = uniform.sample(&mut rng);
        writeln!(f, "{:+e}", random_number).unwrap();
    }
}
