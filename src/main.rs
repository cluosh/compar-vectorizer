#[macro_use]
extern crate nom;
extern crate petgraph;

mod vectorization;

fn main() {
	use std::env;

	let args: Vec<_> = env::args().collect();
	if args.len() < 3 {
		eprintln!("Specify trace and ast file.");
		return;
	}

	if let Err(_) = vectorization::vectorize(&args[1], &args[2]) {
		eprintln!("I/O Error during vectorization.");
	}
}
