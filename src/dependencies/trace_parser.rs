use super::*;
use nom::{digit, types::CompleteStr};
use std::{cmp::Ordering, collections::HashMap, io, str::FromStr};

pub(super) fn read_trace<T>(input: T) -> io::Result<(Vec<StatementInstance>, Vec<Access>)>
where
    T: io::BufRead,
{
    let mut access = Vec::new();
    let mut instances = Vec::new();
    let mut last_statement = 0;
    let mut loops = Vec::new();
    let mut iteration = Vec::new();
    let mut loop_updated = false;

    for line in input.lines() {
        let t = line?
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Could not parse trace line"))?;

        // Deal with different trace outputs
        match t {
            TraceOutput::Access(mut a) => {
                // Begin of new statement instance detected
                if a.statement != last_statement || loop_updated {
                    instances.push(StatementInstance {
                        statement: a.statement,
                        loops: loops.clone(),
                        iteration: iteration.clone(),
                    });

                    last_statement = a.statement;
                    loop_updated = false;
                }

                // Update reference of statement to instance
                a.statement = instances.len() as i32 - 1;
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

                loop_updated = true;
            }
        }
    }

    Ok((instances, access))
}

pub(super) fn split_and_sort_trace(trace: Vec<Access>) -> HashMap<String, Vec<Access>> {
    let mut map: HashMap<String, Vec<Access>> = HashMap::new();

    for a in trace {
        map.entry(a.var.to_owned())
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
            Err(_) => Err(TraceError::ParseAccessError),
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

named!(i32_digit<CompleteStr, i32>, map_res!(
    digit,
    |c: CompleteStr| FromStr::from_str(*c)
));

named!(category<CompleteStr, Category>, alt!(
    tag!("DEF") => { |_| Category::Write } |
    tag!("USE") => { |_| Category::Read }
));

named!(access<CompleteStr, TraceOutput>, ws!(do_parse!(
    statement: i32_digit            >>
    var: take_until!(" ")           >>
    category: category              >>
    indices: many1!(ws!(i32_digit)) >>
    (TraceOutput::Access(Access { statement, category, var: var.to_string(), indices }))
)));

named!(access_no_indices<CompleteStr, TraceOutput>, ws!(do_parse!(
    statement: i32_digit            >>
    var: take_until!(" ")           >>
    category: category              >>
    (TraceOutput::Access(Access { statement, category, var: var.to_string(), indices: Vec::new() }))
)));

named!(loop_begin<CompleteStr, TraceOutput>, ws!(do_parse!(
    label: i32_digit         >>
    _var: take_until!(" ")   >>
    tag!("loop")             >>
    tag!("begin")            >>
    (TraceOutput::LoopBegin(label))
)));

named!(loop_end<CompleteStr, TraceOutput>, ws!(do_parse!(
    _label: i32_digit        >>
    _var: take_until!(" ")   >>
    tag!("loop")             >>
    tag!("end")              >>
    (TraceOutput::LoopEnd)
)));

named!(loop_iteration<CompleteStr, TraceOutput>, ws!(do_parse!(
    _label: i32_digit        >>
    _index: take_until!(" ") >>
    value: i32_digit         >>
    (TraceOutput::LoopUpdate(value))
)));

named!(parse_trace<CompleteStr, TraceOutput>, alt!(
    access | access_no_indices | loop_begin | loop_end | loop_iteration
));
