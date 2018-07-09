use std::{io, str::FromStr, collections::HashMap};
use nom::{digit, types::CompleteStr};
use super::{TraceError, Category, Access};

fn read_trace<T>(input: T) -> Vec<Access>
	where T: io::BufRead
{
	let mut access = Vec::new();

	for line in input.lines() {
    	match line {
    		Ok(l) => if let Ok(a) = Access::from_str(&l) {
    			access.push(a);
    		}
    		Err(_) => break
    	}
    }

	access
}

fn split_and_sort_trace(trace: Vec<Access>) -> HashMap<String, Vec<Access>> {
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

impl FromStr for Access {
	type Err = TraceError;

	fn from_str(trace_line: &str) -> Result<Self, Self::Err> {
		match access(CompleteStr(trace_line)) {
			Ok((_, a)) => Ok(a),
			Err(_) => Err(TraceError::ParseAccessError)
		}
	}
}

named!(u32_digit<CompleteStr, u32>, map_res!(
	digit,
	|c: CompleteStr| FromStr::from_str(*c)
));

named!(category<CompleteStr, Category>, alt!(
	tag!("DEF") => { |_| Category::Write } |
	tag!("USE") => { |_| Category::Read } 
));

named!(access<CompleteStr, Access>, ws!(do_parse!(
	statement: u32_digit            >>
	array: take_until!(" ")         >>
	category: category              >>
	indices: many1!(ws!(u32_digit)) >>
	(Access { statement, category, array: array.to_string(), indices })
)));