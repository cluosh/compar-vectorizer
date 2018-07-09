use super::{Access, Statement, Dependency, DependencyEdge};

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

	fn transition(&mut self, access: Access) -> Option<Dependency> {
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
							let dep = DependencyEdge(statement, write);
							dependency = Some(Dependency::True(dep));
						}

						State::Read {
							statement,
							last_write
						}
					},
					Write => {
						let dep = DependencyEdge(statement, last_read);
						dependency = Some(Dependency::Anti(dep));

						State::Write {
							statement
						}
					}
				}
			}
			State::Write { statement: last_write } => {
				let statement = access.statement;
				let dep = DependencyEdge(statement, last_write);

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