use petgraph::{prelude::*, Graph, algo::tarjan_scc};
use std::{io, fs::File};
use super::*;

pub(super) fn build_graph(statements: Vec<Statement>, deps: Vec<Dependency>)
	-> Graph<u32, Vec<LevelDependency>>
{
	let mut graph = Graph::new();

	statements
		.into_iter()
		.for_each(|s| { graph.add_node(s); });

	for Dependency { edge: DependencyEdge(s1,s2), level_deps } in deps {
		let n1 = NodeIndex::new(s1 as usize - 1);
		let n2 = NodeIndex::new(s2 as usize - 1);
		graph.update_edge(n1, n2, level_deps);
	}

	graph
}

pub(super) fn print_graph(
	graph: &Graph<u32, Vec<LevelDependency>>,
	file_name: &str
) -> Result<(),io::Error>
{
	use std::io::Write;
	
	let f = File::create(file_name)?;
	let mut writer = io::BufWriter::new(f);

	writeln!(writer, "digraph dependencies {{")?;

	let scc = tarjan_scc(graph);
	for (i, subgraph) in scc.into_iter().enumerate() {
		writeln!(writer, "  subgraph cluster_{} {{", i)?;
		for index in subgraph {
			writeln!(writer, "    s{} [label=\"S{}\"];", index.index(), graph[index])?;
		}
		writeln!(writer, "    graph[style=dotted];")?;
		writeln!(writer, "  }}")?;
		writeln!(writer, "")?;
	}

	for edge in graph.raw_edges().iter() {
		writeln!(writer, "  s{} -> ", edge.source().index())?;
		writeln!(writer,"s{} [label = \"", edge.target().index())?;
		for LevelDependency(level, dep) in edge.weight.iter() {
			match dep {
				DependencyType::Anti => write!(writer, " A{}", level)?,
				DependencyType::Output => write!(writer, " O{}", level)?,
				DependencyType::True => write!(writer, " T{}", level)?
			}
		}
		writeln!(writer, "\"];")?;
	}

	writeln!(writer, "}}")?;

	Ok(())
}