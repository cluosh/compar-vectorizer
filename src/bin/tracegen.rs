extern crate petgraph;
extern crate serde_json;
extern crate vectorizer;

use std::{
    env,
    fs::File,
    io::{BufWriter, Read},
};
use vectorizer::{codegen, ir};

fn print_usage(prog_name: &str) {
    eprintln!("Usage: {} project_name", prog_name);
}

fn main() {
    let mut arg_iter = env::args();

    // Program name
    let prog_name = match arg_iter.next() {
        Some(p) => p,
        None => {
            print_usage("./vectorize");
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

    // Open file for tracing output file
    let file = match File::create(&format!("{}.f90", &project_name)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Could not open {}.f90 for writing: {}", &project_name, e);
            return;
        }
    };

    // Generate tracing code
    let writer = BufWriter::new(file);
    match codegen::generate_trace(&ast, writer) {
        Ok(_) => (),
        Err(e) => {
            eprintln!(
                "Could not generate trace program {}.f90: {}",
                &project_name, e
            );
            return;
        }
    }
}
