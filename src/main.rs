#[macro_use]
extern crate nom;

mod depfinder;

fn main() {
	let v1 = vec![3, 2, 3];
	let v2 = vec![3, 2, 4];
	let v3 = vec![3, 2, 5];
	let v4 = vec![3, 6, 3];
	let v5 = vec![7, 2, 3];
	let mut test = vec![v1, v2, v3, v4, v5];
	test.sort();

	println!("{:?}", test);
    // use std::io::{self, BufRead, BufReader};
    // use depfinder::trace_parser::Access;
    // use std::str::FromStr;

    // let stdin = BufReader::new(io::stdin());
    // for line in stdin.lines() {
    // 	match line {
    // 		Ok(l) => if let Ok(a) = Access::from_str(&l) {
    // 			println!("{:?}", &a.indices);
    // 		}
    // 		Err(_) => break
    // 	}
    // }
}
