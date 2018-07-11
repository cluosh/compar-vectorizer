mod trace_parser;
mod trace_deps;
mod graph;
mod ir_parse;

use std::{io, collections::{HashSet, HashMap}};
use petgraph::{Graph, algo::{tarjan_scc, is_cyclic_directed}};
use self::trace_parser::{read_trace, split_and_sort_trace};
use self::trace_deps::find_deps_for_var;
use self::graph::{build_graph, print_graph};

pub use self::ir_parse::*;

pub fn vectorize(trace_file: &str, ast_file: &str) -> Result<(), io::Error> {
	use std::io::Read;
	use std::fs::File;

	let trace_file = File::open(trace_file)?;
	let input = io::BufReader::new(trace_file);

	let (instances, trace) = read_trace(input);
	let trace = split_and_sort_trace(trace);

	// Find dependencies
	let mut dependencies = HashMap::new();
	for var in trace.values() {
		find_deps_for_var(var, &instances, &mut dependencies);
	}

	// Find statements in instances
	let mut statements = instances.iter()
		.map(|i| i.statement)
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();
	statements.sort();

	// Find statement loop info in instances
	let stat_loops = instances.iter()
		.map(|i| (i.statement, i.loops.clone()))
		.collect::<HashMap<_,_>>();

	// Collect dependencies into array
	let dependencies = dependencies
		.into_iter()
		.map(|(edge, level_deps)| {
			let mut level_deps: Vec<_>= level_deps.into_iter().collect();
			level_deps.sort_by_key(|d| d.0);
			Dependency { edge, level_deps }
		})
		.collect::<Vec<_>>();

	// Read AST
	let ast_file = File::open(ast_file)?;
	let mut input = io::BufReader::new(ast_file);
	let mut ast = String::new();
    input.read_to_string(&mut ast).unwrap();
    ast.push_str(" $");
    let ast = ast.replace('\n', " ");
    let ast = ast.replace('\t', " ");

    let ast = match parse_ast(&ast) {
    	Ok((_, ast)) => ast,
    	Err(_) => {
    		eprintln!("Could not parse AST.");
    		return Ok(());
    	}
    };

    let mut loop_map = HashMap::new();
    ast_loops(&ast.statements.0, &mut loop_map);
    let mut stat_map = HashMap::new();
    ast_statements(&ast.statements.0, &mut stat_map);

	// Build layered dependence graph
	let graph = build_graph(statements, dependencies);
	print_graph(&graph, "out_deps.dot")?;

	// Vectorize
	allen_kennedy(graph, &ast, &loop_map, &stat_map, &stat_loops, 1);

	Ok(())
}

fn allen_kennedy(
	graph: Graph<u32, Vec<LevelDependency>>,
	ast: &Ast,
	loop_map: &HashMap<i32, &Loop>,
	stat_map: &HashMap<i32, &Assign>,
	stat_loops: &HashMap<u32, Vec<LoopLabel>>,
	loop_level: u32
) {
	// Filter dependencies for adequate loop level
	let graph = graph.filter_map(|_, n| {
		Some(*n)
	}, |_, e| {
		// Filter dependencies by depth
		let mut new_ldeps = Vec::new();
		for ldep in e.iter() {
			if ldep.0 == 0 || ldep.0 >= loop_level {
				new_ldeps.push(ldep.clone());
			}
		}

		if new_ldeps.len() > 0 {
			Some(new_ldeps)
		} else {
			None
		}
	});

	// Calculate SCCs
	let scc = tarjan_scc(&graph);

	// Loop over SCCs in topological order
	for sub_nodes in scc.into_iter().rev() {
		// Store nodes of subgraph in set
		let mut node_set = HashSet::new();
		for n in sub_nodes.iter() {
			node_set.insert(n.index());
		}

		// Get actual subgraph
		let subgraph = graph.filter_map(|i,n| {
			if node_set.contains(&i.index()) {
				Some(*n)
			} else {
				None
			}
		}, |_, e| Some(e.clone()));

		if is_cyclic_directed(&subgraph) {
			println!("for i_{0:} = lb_{0:} to ub_{0:}", loop_level);
			allen_kennedy(subgraph, ast, loop_map, stat_map, stat_loops, loop_level + 1);
			println!("endfor");
		} else {
			for node in sub_nodes.iter() {
				if let Some(stat) = graph.node_weight(*node) {
					let loops = &stat_loops[stat];

					
				}
			}
			// GENERATE VECTOR CODE "S(i1,...,ic-1,lb_c:ub_c,...,lb_n:ub_n)"
			println!("GENERATE VECTOR CODE");
		}
	} 
}

#[derive(Debug)]
enum TraceError {
	ParseAccessError
}

type Statement = u32;
type LoopLabel = u32;
pub type Level = u32;

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
	array: String,
	category: Category,
	indices: Vec<u32>
}

#[derive(Debug,Hash,Eq,PartialEq,Clone)]
pub enum DependencyType {
	True,
	Anti,
	Output
}

#[derive(Debug,Hash,Eq,PartialEq,Clone)]
pub struct LevelDependency(Level, DependencyType);

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