use codegen::{Codegen, Generator, Vectorizer};
use dependencies::{Level, LevelDependency, LoopLabel, Statement};
use ir::{Assign, Ast, Loop};
use petgraph::{
    algo::{is_cyclic_directed, tarjan_scc},
    Graph,
};
use std::collections::{HashMap, HashSet};
use std::io;

pub fn vectorize<W>(
    graph: &Graph<Statement, Vec<LevelDependency>>,
    ast: &Ast,
    writer: W,
    fold: bool,
) -> io::Result<()>
where
    W: io::Write,
{
    let mut loop_map = HashMap::new();
    ast_loops(&ast.statements.0, &mut loop_map);

    let mut loop_cur = Vec::new();
    let mut stat_map = HashMap::new();
    let mut stat_lps = HashMap::new();
    ast_statements(
        &ast.statements.0,
        &mut loop_cur,
        &mut stat_map,
        &mut stat_lps,
    );

    // Generate code
    let mut cg: Codegen<Vectorizer<_>, _> = if fold {
        Codegen::new_folding(writer)
    } else {
        Codegen::new(writer)
    };
    cg.generate_header(ast)?;
    allen_kennedy(&mut cg, graph, ast, &loop_map, &stat_map, &stat_lps, 0)?;
    cg.generate_footer(ast)
}

fn allen_kennedy<'a, G, W>(
    codegen: &mut Codegen<G, W>,
    graph: &Graph<Statement, Vec<LevelDependency>>,
    ast: &'a Ast,
    loop_map: &HashMap<LoopLabel, &'a Loop>,
    stat_map: &HashMap<Statement, &'a Assign>,
    stat_lps: &HashMap<Statement, Vec<LoopLabel>>,
    c: Level,
) -> io::Result<()>
where
    G: Generator<'a, G, W>,
    W: io::Write,
{
    // Filter dependencies for adequate loop level
    let graph = graph.filter_map(
        |_, n| Some(*n),
        |_, e| {
            // Filter dependencies by depth
            let mut new_ldeps = Vec::new();
            for ldep in e.iter() {
                if ldep.0 == 0 || ldep.0 > c {
                    new_ldeps.push(ldep.clone());
                }
            }

            if new_ldeps.len() > 0 {
                Some(new_ldeps)
            } else {
                None
            }
        },
    );

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
        let subgraph = graph.filter_map(
            |i, n| {
                if node_set.contains(&i.index()) {
                    Some(*n)
                } else {
                    None
                }
            },
            |_, e| Some(e.clone()),
        );

        if is_cyclic_directed(&subgraph) {
            if let Some(n) = sub_nodes.first() {
                let stat = graph.node_weight(*n).unwrap_or(&-1);
                let l = match stat_lps.get(stat) {
                    Some(l) => l,
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Could not lookup loops for statement {}", stat),
                        ))
                    }
                };

                if l.len() <= c as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Not enough loops for statement {}", stat),
                    ));
                }

                let l = match loop_map.get(&l[c as usize]) {
                    Some(l) => l,
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Could not lookup loop with label {}", l[c as usize]),
                        ))
                    }
                };

                codegen.generate_loop_vec_start(l, c)?;
                allen_kennedy(codegen, &subgraph, ast, loop_map, stat_map, stat_lps, c + 1)?;
                codegen.generate_loop_vec_end(l, c)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("No nodes in subgraph at level {}", c + 1),
                ));
            }
        } else {
            for node in sub_nodes.iter() {
                if let Some(stat) = graph.node_weight(*node) {
                    let assign = match stat_map.get(stat) {
                        Some(s) => s,
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("Could not lookup statement {}", stat),
                            ))
                        }
                    };
                    let loops = match stat_lps.get(stat) {
                        Some(l) => l,
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("Could not lookup loops for statement {}", stat),
                            ))
                        }
                    };

                    let mut stat_loops = HashMap::new();
                    if loops.len() > c as usize {
                        for label in &loops[c as usize..] {
                            let l = match loop_map.get(label) {
                                Some(l) => *l,
                                None => {
                                    return Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        format!("Could not lookup loop with label: {}", label),
                                    ))
                                }
                            };
                            stat_loops.insert(l.var.to_owned(), l);
                        }
                    }

                    codegen.set_loop_data(stat_loops);
                    codegen.generate_assignment(assign, c as u8)?;
                }
            }
        }
    }

    Ok(())
}

fn ast_loops<'a>(statements: &'a [::ir::Statement], loop_map: &mut HashMap<LoopLabel, &'a Loop>) {
    for s in statements {
        if let ::ir::Statement::Loop(l) = s {
            loop_map.insert(l.label, l);
            ast_loops(&l.statements.0, loop_map);
        }
    }
}

pub fn ast_statements<'a>(
    statements: &'a [::ir::Statement],
    loop_cur: &mut Vec<LoopLabel>,
    stat_map: &mut HashMap<Statement, &'a Assign>,
    stat_lps: &mut HashMap<Statement, Vec<LoopLabel>>,
) {
    for s in statements {
        match s {
            ::ir::Statement::Loop(l) => {
                loop_cur.push(l.label);
                ast_statements(&l.statements.0, loop_cur, stat_map, stat_lps);
                loop_cur.pop();
            }
            ::ir::Statement::Assignment(a) => {
                stat_map.insert(a.label, a);
                stat_lps.insert(a.label, loop_cur.clone());
            }
            ::ir::Statement::If(_) => {
                eprintln!("IF-Statements not supported in Vectorization");
            }
        }
    }
}
