use super::{Access, Statement, Dependency, DependencyEdge};

pub(super) fn find_deps_for_var(sorted_access: &[Access]) {
	let mut fsm = StateMachine::new();

	let mut iter = sorted_access.iter();
	let mut last = match iter.next() {
		Some(l) => l,
		None => return
	};

	while let Some(access) = iter.next() {
		if let Some(dep) = fsm.transition(last) {
			println!("{:?}", dep);
		}

		if access.indices != last.indices {
			fsm = StateMachine::new();
		}

		last = access;
	}
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

	fn transition(&mut self, access: &Access) -> Option<Dependency> {
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
							dependency = Some(Dependency::True(dep));
						}

						State::Read {
							statement,
							last_write
						}
					},
					Write => {
						let dep = DependencyEdge(last_read, statement);
						dependency = Some(Dependency::Anti(dep));

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
						dependency = Some(Dependency::True(dep));

						State::Read {
							statement,
							last_write: Some(last_write)
						}
					},
					Write => {
						dependency = Some(Dependency::Output(dep));

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