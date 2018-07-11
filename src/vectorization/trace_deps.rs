use std::collections::{HashMap, HashSet};
use super::*;

pub(super) fn find_deps_for_var(
	sorted_access: &[Access],
	instances: &[StatementInstance],
	deps: &mut HashMap<DependencyEdge, HashSet<LevelDependency>>
) {
	let mut fsm = StateMachine::new();
	let mut iter = sorted_access.iter();
	let mut last = match iter.next() {
		Some(l) => l,
		None => return
	};

	while let Some(access) = iter.next() {
		if let Some((edge, dep)) = fsm.transition(last) {
			let (si1,si2) = to_instances(edge, instances);
			let edge = DependencyEdge(si1.statement, si2.statement);

			deps.entry(edge)
				.and_modify(|set| {
					let levels = find_levels(si1, si2);
					for l in levels {
						set.insert(LevelDependency(l, dep.clone()));
					}
				})
				.or_insert_with(|| {
					let mut set = HashSet::new();

					let levels = find_levels(si1, si2);
					for l in levels {
						set.insert(LevelDependency(l, dep.clone()));
					}

					set
				});
		}

		if access.indices != last.indices {
			fsm = StateMachine::new();
		}

		last = access;
	}
}

fn to_instances(dep: DependencyEdge, instances: &[StatementInstance])
	-> (&StatementInstance, &StatementInstance)
{
	let DependencyEdge(i1, i2) = dep;
	(&instances[i1 as usize], &instances[i2 as usize])
}

fn find_levels(s1: &StatementInstance, s2: &StatementInstance) -> Vec<u32> {
	let maxlevel = max_common_level(s1, s2);

	// Find loop carried dependencies
	let mut levels: Vec<u32> = s1.iteration
		.iter()
		.zip(s2.iteration.iter())
		.take(maxlevel)
		.enumerate()
		.filter(|(_, (j,k))| j < k)
		.map(|(i, _)| i as u32 + 1)
		.collect();

	// If we have no loop carried dependencies,
	// dependency is loop independent
	if levels.is_empty() {
		levels.push(0);
	}

	levels
}

fn max_common_level(s1: &StatementInstance, s2: &StatementInstance) -> usize {
	let loops = s1.loops.iter().zip(s2.loops.iter());
	let mut counter = 0;
	for (l1,l2) in loops {
		if l1 != l2 {
			break
		}

		counter += 1;
	}

	counter
}

enum State {
	Uninitialized,
	Read { statement: Statement, last_write: Option<Statement> },
	Write { statement: Statement }
}

struct StateMachine {
	state: State
}

impl StateMachine {
	fn new() -> Self {
		let state = State::Uninitialized;
		StateMachine { state }
	}

	fn transition(&mut self, access: &Access) -> Option<(DependencyEdge, DependencyType)> {
		use super::Category::*;

		let mut dependency = None;

		self.state = match self.state {
			State::Uninitialized => {
				let statement = access.statement;

				match access.category {
					Read => State::Read {
						statement,
						last_write: None
					},
					Write => State::Write {
						statement
					}
				}
			}
			State::Read { last_write, statement: last_read } => {
				let statement = access.statement;

				match access.category {
					Read => {
						if let Some(write) = last_write {
							let dep = DependencyEdge(write, statement);
							dependency = Some((dep, DependencyType::True));
						}

						State::Read {
							statement,
							last_write
						}
					},
					Write => {
						if statement != last_read {
							let dep = DependencyEdge(last_read, statement);
							dependency = Some((dep, DependencyType::Anti));
						}

						State::Write {
							statement
						}
					}
				}
			}
			State::Write { statement: last_write } => {
				let statement = access.statement;
				let dep = DependencyEdge(last_write, statement);

				match access.category {
					Read => {
						dependency = Some((dep, DependencyType::True));

						State::Read {
							statement,
							last_write: Some(last_write)
						}
					},
					Write => {
						dependency = Some((dep, DependencyType::Output));

						State::Write {
							statement
						}
					}
				}
			}
		};

		dependency
	}
}