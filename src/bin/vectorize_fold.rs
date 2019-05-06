extern crate vectorizer;
extern crate serde_json;
extern crate petgraph;

use std::{env, fs::File, io::{BufReader, Read, BufWriter}};
use vectorizer::{ir, dependencies::{Statement, LevelDependency}, vectorization};

fn print_usage(prog_name: &str) {
	eprintln!("Usage: {} project_name", prog_name);
}

fn main() {
	let mut arg_iter = env::args();

	// Program name
	let prog_name = match arg_iter.next() {
		Some(p) => p,
		None => {
			print_usage("./vectorize_fold");
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

	// Open dependency graph file
	let graph_file = match File::open(&format!("{}.graph", &project_name)) {
		Ok(f) => f,
		Err(e) => {
			eprintln!("Could not open {}.graph: {}", &project_name, e);
			return;
		}
	};
	let inp = BufReader::new(graph_file);

	// Deserialize dependency graph
	let graph: petgraph::Graph<Statement, Vec<LevelDependency>> = match serde_json::from_reader(inp) {
		Ok(g) => g,
		Err(e) => {
			eprintln!("Could not deserialize {}.graph: {}", &project_name, e);
			return;
		}
	};

	// Open AST file
	let mut ast_file = match File::open(&format!("{}.ast", &project_name)) {
		Ok(f) => f,
		Err(e) => {
			eprintln!("Could not open {}.ast: {}", &project_name, e);
			return;
		}
	};

	// Read AST
	let mut ir_text = String::new();
	if let Err(e) = ast_file.read_to_string(&mut ir_text) {
		eprintln!("Error while reading IR from {}.ast: {}", &project_name, e);
	}

	// Fix IR input for parser
	ir_text.push_str(" $");
    let ir_text = ir_text.replace('\n', " ");
    let ir_text = ir_text.replace('\t', " ");

    // Parse AST
	let ast = match ir::parse_ast(&ir_text) {
		Ok((_, ast)) => ast,
		Err(e) => {
			eprintln!("Could not parse {}.ast: {}", &project_name, e);
			return;
		}
	};

	// Open file for vectorization output
	let file = match File::create(&format!("{}_vectorized_fold.f90", &project_name)) {
		Ok(f) => f,
		Err(e) => {
			eprintln!("Could not open {}_vectorized_foldc.f90 for writing: {}", &project_name, e);
			return;
		}
	};

	// Generate vector code
	let writer = BufWriter::new(file);
	match vectorization::vectorize(&graph, &ast, writer, true) {
		Ok(_) => (),
		Err(e) => {
			eprintln!("Could not vectorize {}: {}", &project_name, e);
			return;
		}
	}
}