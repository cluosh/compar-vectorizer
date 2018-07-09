#[derive(Debug)]
enum TraceError {
	ParseAccessError
}

type Statement = u32;

#[derive(Debug)]
struct DependencyEdge(u32, u32);

#[derive(Debug)]
enum Category {
	Read,
	Write
}

#[derive(Debug)]
struct Access {
	statement: Statement,
	array: String,
	category: Category,
	indices: Vec<u32>
}

#[derive(Debug)]
enum Dependency {
	True(DependencyEdge),
	Anti(DependencyEdge),
	Output(DependencyEdge)
}

mod trace_parser;
mod trace_deps;