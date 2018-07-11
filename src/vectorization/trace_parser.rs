use std::{io, str::FromStr, collections::HashMap, cmp::Ordering};
use nom::{digit, types::CompleteStr};
use super::*;

pub(super) fn read_trace<T>(input: T) -> (Vec<StatementInstance>, Vec<Access>)
	where T: io::BufRead
{
	let mut access = Vec::new();
	let mut instances = Vec::new();
	let mut last_statement = 0;
	let mut loops = Vec::new();
	let mut iteration = Vec::new();

	for line in input.lines() {
		let t = match line {
			Ok(l) => l.parse(),
			Err(_) => break
		};

		// Parse trace output line
		let t = match t {
			Ok(out) => out,
			Err(_) => {
				eprintln!("Could not parse trace line.");
				continue;
			}
		};

		// Deal with different trace outputs
		match t {
			TraceOutput::Access(mut a) => {
				// Begin of new statement instance detected
		    	if a.statement != last_statement {
		    		instances.push(StatementInstance {
		    			statement: a.statement,
		    			loops: loops.clone(),
		    			iteration: iteration.clone()
		    		});

		    		last_statement = a.statement;
		    	}

		    	// Update reference of statement to instance
		    	a.statement = instances.len() as u32 - 1;
		    	access.push(a);
			}
			TraceOutput::LoopBegin(label) => {
				loops.push(label);
				iteration.push(0);
			}
			TraceOutput::LoopEnd => {
				loops.pop();
				iteration.pop();
			}
			TraceOutput::LoopUpdate(index) => {
				if let Some(i) = iteration.last_mut() {
					*i = index
				}
			}
		}
    }

	(instances, access)
}

pub(super) fn split_and_sort_trace(trace: Vec<Access>) -> HashMap<String, Vec<Access>> {
	let mut map: HashMap<String, Vec<Access>> = HashMap::new();

	for a in trace {
		map.entry(a.array.to_owned())
			.and_modify(|v| v.push(a.clone()))
			.or_insert(vec![a]);
	}

	for vec in map.values_mut() {
		vec.sort();
	}

	map
}

impl FromStr for TraceOutput {
	type Err = TraceError;

	fn from_str(trace_line: &str) -> Result<Self, Self::Err> {
		match parse_trace(CompleteStr(trace_line)) {
			Ok((_, t)) => Ok(t),
			Err(_) => Err(TraceError::ParseAccessError)
		}
	}
}

impl Ord for Access {
	fn cmp(&self, other: &Access) -> Ordering {
		self.indices.cmp(&other.indices)
	}
}

impl PartialOrd for Access {
	fn partial_cmp(&self, other: &Access) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for Access {
	fn eq(&self, other: &Access) -> bool {
		self.indices == other.indices
	}
}

named!(u32_digit<CompleteStr, u32>, map_res!(
	digit,
	|c: CompleteStr| FromStr::from_str(*c)
));

named!(i32_digit<CompleteStr, i32>, map_res!(
	digit,
	|c: CompleteStr| FromStr::from_str(*c)
));

named!(category<CompleteStr, Category>, alt!(
	tag!("DEF") => { |_| Category::Write } |
	tag!("USE") => { |_| Category::Read } 
));

named!(access<CompleteStr, TraceOutput>, ws!(do_parse!(
	statement: u32_digit            >>
	array: take_until!(" ")         >>
	category: category              >>
	indices: many1!(ws!(u32_digit)) >>
	(TraceOutput::Access(Access { statement, category, array: array.to_string(), indices }))
)));

named!(loop_begin<CompleteStr, TraceOutput>, ws!(do_parse!(
	label: u32_digit         >>
	_var: take_until!(" ")   >>
	tag!("loop")             >>
	tag!("begin")            >>
	(TraceOutput::LoopBegin(label))
)));

named!(loop_end<CompleteStr, TraceOutput>, ws!(do_parse!(
	_label: u32_digit        >>
	_var: take_until!(" ")   >>
	tag!("loop")             >>
	tag!("end")              >>
	(TraceOutput::LoopEnd)
)));

named!(loop_iteration<CompleteStr, TraceOutput>, ws!(do_parse!(
	_label: u32_digit        >>
	_index: take_until!(" ") >>
	value: i32_digit         >>
	(TraceOutput::LoopUpdate(value))
)));

named!(parse_trace<CompleteStr, TraceOutput>, alt!(
	access | loop_begin | loop_end | loop_iteration
));