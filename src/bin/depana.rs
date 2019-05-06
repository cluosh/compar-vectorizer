extern crate vectorizer;
extern crate serde_json;
extern crate petgraph;

use std::{env, fs::File, io::{BufReader, BufWriter}};
use vectorizer::dependencies;

fn print_usage(prog_name: &str) {
	eprintln!("Usage: {} project_name", prog_name);
}

fn main() {
	let mut arg_iter = env::args();

	// Program name
	let prog_name = match arg_iter.next() {
		Some(p) => p,
		None => {
			print_usage("./depana");
			return;
		}
	};

	// Project name
	let project_name = match arg_iter.next() {
		Some(n) => n,
		None => {
			print_usage(&prog_name);
			return;
		}
	};

	// Open trace file
	let trace = match File::open(&format!("{}.trace", &project_name)) {
		Ok(f) => f,
		Err(e) => {
			eprintln!("Could not open {}.trace: {}", &project_name, e);
			return;
		}
	};
	let inp = BufReader::new(trace);

	// Analyze dependencies
	let graph = match dependencies::find_dependencies(inp) {
		Ok(g) => g,
		Err(e) => {
			eprintln!("Could not find dependencies: {}", e);
			return;
		}
	};

	// Print graph to dot file
	match dependencies::print_graph(&graph, &format!("{}.dot", &project_name)) {
		Ok(_) => (),
		Err(e) => {
			eprintln!("Could not print graph to {}.dot: {}", &project_name, e);
			return;
		}
	}

	// Serialize graph to graph file
	let graph_file = match File::create(&format!("{}.graph", &project_name)) {
		Ok(f) => f,
		Err(e) => {
			eprintln!("Could not open {}.graph file for writing: {}", &project_name, e);
			return;
		}
	};
	let writer = BufWriter::new(graph_file);
	match serde_json::to_writer(writer, &graph) {
		Ok(_) => (),
		Err(e) => {
			eprintln!("Could not serialize graph to {}.graph: {}", &project_name, e);
			return;
		}
	}
}