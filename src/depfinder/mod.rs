mod trace_parser;
mod trace_deps;

use std::io;
use self::trace_parser::{read_trace, split_and_sort_trace};
use self::trace_deps::find_deps_for_var;

pub fn find_dependencies<T>(input: T)
	where T: io::BufRead
{
	let trace = read_trace(input);
	let trace = split_and_sort_trace(trace);

	for var in trace.values() {
		find_deps_for_var(var);
	}
}

#[derive(Debug)]
enum TraceError {
	ParseAccessError
}

type Statement = u32;

#[derive(Debug)]
struct DependencyEdge(u32, u32);

#[derive(Debug,Clone,Eq,PartialEq)]
enum Category {
	Read,
	Write
}

#[derive(Debug,Clone,Eq)]
struct Access {
	statement: Statement,
	array: String,
	category: Category,
	indices: Vec<u32>
}

#[derive(Debug)]
enum Dependency {
	True(DependencyEdge),
	Anti(DependencyEdge),
	Output(DependencyEdge)
}