use super::*;
use petgraph::{algo::tarjan_scc, prelude::*, Graph};
use std::{fs::File, io};

pub(super) fn build_graph(
    statements: Vec<Statement>,
    deps: Vec<Dependency>,
) -> Graph<Statement, Vec<LevelDependency>> {
    let mut graph = Graph::new();

    statements.into_iter().for_each(|s| {
        graph.add_node(s);
    });

    for Dependency {
        edge: DependencyEdge(s1, s2),
        level_deps,
    } in deps
    {
        let n1 = NodeIndex::new(s1 as usize - 1);
        let n2 = NodeIndex::new(s2 as usize - 1);
        graph.update_edge(n1, n2, level_deps);
    }

    graph
}

pub fn print_graph(
    graph: &Graph<Statement, Vec<LevelDependency>>,
    file_name: &str,
) -> Result<(), io::Error> {
    use std::io::Write;

    let f = File::create(file_name)?;
    let mut writer = io::BufWriter::new(f);

    writeln!(writer, "digraph dependencies {{")?;

    let scc = tarjan_scc(graph);
    for (i, subgraph) in scc.into_iter().enumerate() {
        writeln!(writer, "  subgraph cluster_{} {{", i)?;
        for index in subgraph {
            writeln!(writer, "    s{0:} [label=\"S{0:}\"];", graph[index])?;
        }
        writeln!(writer, "    graph[style=dotted];")?;
        writeln!(writer, "  }}")?;
        writeln!(writer, "")?;
    }

    for edge in graph.raw_edges().iter() {
        write!(writer, "  s{} -> ", graph[edge.source()])?;
        write!(writer, "s{} [label=\"", graph[edge.target()])?;
        for LevelDependency(level, dep) in edge.weight.iter() {
            match dep {
                DependencyType::Anti => write!(writer, " A{}", level)?,
                DependencyType::Output => write!(writer, " O{}", level)?,
                DependencyType::True => write!(writer, " T{}", level)?,
            }
        }
        writeln!(writer, "\"];")?;
    }

    writeln!(writer, "}}")?;

    Ok(())
}
