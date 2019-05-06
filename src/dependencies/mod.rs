mod trace_parser;
mod deps;
mod graph;

use std::{io, collections::{HashMap, HashSet}};
use petgraph::Graph;

pub use self::graph::print_graph;

pub fn find_dependencies<T>(inp: T) -> io::Result<(Graph<Statement, Vec<LevelDependency>>)>
	where T: io::BufRead
{
	let (instances, access) = trace_parser::read_trace(inp)?;
	let access = trace_parser::split_and_sort_trace(access);

	// Find dependencies
	let mut dependencies = HashMap::new();
	for var in access.values() {
		deps::find_deps_for_var(var, &instances, &mut dependencies);
	}

	// Find statements in instances
	let mut statements = instances.iter()
		.map(|i| i.statement)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();
	statements.sort();

	// Collect dependencies into array
	let dependencies = dependencies
		.into_iter()
		.map(|(edge, level_deps)| {
			let mut level_deps: Vec<_>= level_deps.into_iter().collect();
			level_deps.sort_by_key(|d| d.0);
			Dependency { edge, level_deps }
		})
		.collect::<Vec<_>>();

	// Build layered dependence graph
	Ok(graph::build_graph(statements, dependencies))
}

#[derive(Debug)]
enum TraceError {
	ParseAccessError
}

pub type Statement = i32;
pub type LoopLabel = i32;
pub type Level = i32;

#[derive(Debug,Hash,Eq,PartialEq,Clone)]
struct DependencyEdge(Statement,Statement);

#[derive(Debug,Clone,Eq,PartialEq)]
enum Category {
	Read,
	Write
}

#[derive(Debug,Clone,Eq)]
struct Access {
	statement: Statement,
	var: String,
	category: Category,
	indices: Vec<i32>
}

#[derive(Debug,Hash,Eq,PartialEq,Clone,Serialize,Deserialize)]
pub enum DependencyType {
	True,
	Anti,
	Output
}

#[derive(Debug,Hash,Eq,PartialEq,Clone,Serialize,Deserialize)]
pub struct LevelDependency(pub Level, pub DependencyType);

#[derive(Debug)]
struct Dependency {
	edge: DependencyEdge,
	level_deps: Vec<LevelDependency>
}

#[derive(Debug)]
struct StatementInstance {
	statement: Statement,
	loops: Vec<LoopLabel>,
	iteration: Vec<i32>
}

#[derive(Debug)]
enum TraceOutput {
	Access(Access),
	LoopBegin(LoopLabel),
	LoopEnd,
	LoopUpdate(i32),
}