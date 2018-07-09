#[macro_use]
extern crate nom;
extern crate petgraph;

mod depfinder;

fn main() {
    use std::io::{self, BufReader};

    let input = BufReader::new(io::stdin());
    depfinder::find_dependencies(input);
}
