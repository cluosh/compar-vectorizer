use nom::{double_s, digit};
use std::{str::FromStr, collections::HashMap};

pub fn ast_loops<'a>(
	statements: &'a[Statement],
	loop_map: &mut HashMap<i32, &'a Loop>
) {
	for s in statements {
		if let Statement::Loop(l) = s {
			loop_map.insert(l.label, l);
			ast_loops(&l.statements.0, loop_map);
		}
	}
}

pub fn ast_statements<'a>(
	statements: &'a[Statement],
	stat_map: &mut HashMap<i32, &'a Assign>
) {
	for s in statements {
		match s {
			Statement::Loop(l) => ast_statements(&l.statements.0, stat_map),
			Statement::Assignment(a) => { stat_map.insert(a.label, a); }
		}
	}
}

#[derive(Debug)]
pub struct Ast {
	pub name: String,
	pub vardef: Vec<Definition>,
	pub statements: StatementList
}

#[derive(Debug)]
pub struct Definition {
	pub name: String,
	pub dimensions: Vec<(i32,i32)>,
	pub dtype: DefinitionType
}

#[derive(Debug)]
pub enum DefinitionType {
	Real,
	Integer
}

#[derive(Debug)]
pub struct StatementList(pub Vec<Statement>);

#[derive(Debug)]
pub enum Statement {
	Assignment(Assign),
	Loop(Loop)
}

#[derive(Debug)]
pub struct Assign {
	pub label: i32,
	pub lhs: Variable,
	pub rhs: Expression
}

#[derive(Debug)]
pub struct Loop {
	pub label: i32,
	pub var: String,
	pub lower: i32,
	pub upper: i32,
	pub step: i32,
	pub statements: StatementList
}

#[derive(Debug)]
pub struct Variable {
	pub name: String,
	pub indices: Vec<Expression>
}

#[derive(Debug)]
pub enum Expression {
	Integer(i32),
	Real(f64),
	BinOp(Box<BinOp>),
	Variable(Variable)
}

#[derive(Debug)]
pub enum BinOpType {
	Plus,
	Minus,
	Mul,
	Div
}

#[derive(Debug)]
pub struct BinOp {
	pub op: BinOpType,
	pub left: Expression,
	pub right: Expression
}

named!(i32_digit<&str, i32>, map_res!(
	digit,
	FromStr::from_str
));

named!(parse_real<&str,Expression>, ws!(do_parse!(
	tag!("FLOAT")   >>
	value: double_s >>
	(Expression::Real(value))
)));

named!(parse_int<&str,Expression>, ws!(do_parse!(
	tag!("INT")      >>
	value: i32_digit >>
	(Expression::Integer(value))
)));

named!(parse_entry<&str,String>, ws!(do_parse!(
	tag!("ENTRY")           >>
	value: take_until!(" ") >>
	(value.to_owned())
)));

named!(parse_expr<&str,Expression>, ws!(do_parse!(
	tag!("EXPR")        >>
	value: alt!(
		parse_real     |
		parse_int      |
		parse_binop    |
		parse_var_expr |
		parse_expr)    >>
	(value)
)));

named!(parse_exprlist<&str,Vec<Expression>>, ws!(do_parse!(
	tag!("EXPRLIST")                >>
	expressions: many1!(parse_expr) >>
	tag!("/EXPRLIST")               >>
	(expressions)
)));

named!(parse_var_indices<&str,Variable>, ws!(do_parse!(
	tag!("VAR")                   >>
	name: parse_entry             >>
	indices: parse_exprlist       >>
	(Variable { name, indices})
)));

named!(parse_var_no_indices<&str,Variable>, ws!(do_parse!(
	tag!("VAR")                   >>
	name: parse_entry             >>
	(Variable { name, indices: Vec::new() })
)));

named!(parse_var<&str,Variable>, ws!(do_parse!(
	var: alt!(parse_var_indices | parse_var_no_indices) >>
	(var)
)));

named!(parse_var_expr<&str,Expression>, ws!(do_parse!(
	value: parse_var >>
	(Expression::Variable(value))
)));

named!(parse_binoptype<&str,BinOpType>, ws!(alt!(
	map!(tag!("+"), |_| BinOpType::Plus) |
	map!(tag!("-"), |_| BinOpType::Minus) |
	map!(tag!("*"), |_| BinOpType::Mul) |
	map!(tag!("/"), |_| BinOpType::Div)
)));

named!(parse_binop<&str,Expression>, ws!(do_parse!(
	tag!("BINOP")       >>
	op: parse_binoptype >>
	left: parse_expr    >>
	right: parse_expr   >>
	(Expression::BinOp(Box::new(BinOp { op, left, right })))
)));

named!(parse_assign<&str,Statement>, ws!(do_parse!(
	tag!("ASSIGN")   >>
	tag!("@")        >>
	label: i32_digit >>
	lhs: parse_var   >>
	rhs: parse_expr  >>
	(Statement::Assignment(Assign { label, lhs, rhs }))
)));

named!(parse_loop_lower<&str,i32>, ws!(do_parse!(
	tag!("EXPR")     >>
	tag!("INT")      >>
	value: i32_digit >>
	(value)
)));

named!(parse_loop_upper_step<&str,(i32,i32)>, ws!(do_parse!(
	tag!("EXPR")     >>
	tag!("BINOP")    >>
	tag!("-")        >>
	tag!("EXPR")     >>
	tag!("INT")      >>
	upper: i32_digit >>
	tag!("EXPR")     >>
	tag!("INT")      >>
	step: i32_digit  >>
	((upper, step))
)));

named!(parse_loop_step<&str,Statement>, ws!(do_parse!(
	tag!("FOR")                       >>
	tag!("@")                         >>
	label: i32_digit                  >>
	var: parse_entry                  >>
	lower: parse_loop_lower           >>
	upper_step: parse_loop_upper_step >>
	statements: parse_stmtlist        >>
	(Statement::Loop(Loop {
		label, var, lower,
		upper: upper_step.0,
		step: - upper_step.1,
		statements
	}))
)));

named!(parse_loop_no_step<&str,Statement>, ws!(do_parse!(
	tag!("FOR")                >>
	tag!("@")                  >>
	label: i32_digit           >>
	var: parse_entry           >>
	lower: parse_loop_lower    >>
	upper: parse_loop_lower    >>
	statements: parse_stmtlist >>
	(Statement::Loop(Loop { label, var, lower, upper, step: 1 , statements }))
)));

named!(parse_loop<&str,Statement>, ws!(do_parse!(
	loops: alt!(parse_loop_step | parse_loop_no_step) >>
	(loops)
)));

named!(parse_stmtlist<&str,StatementList>, ws!(do_parse!(
	tag!("STMTLIST")                              >>
	list: many0!(alt!(parse_assign | parse_loop)) >>
	tag!("/STMTLIST")                             >>
	(StatementList(list))
)));

named!(parse_dimension<&str,(i32,i32)>, ws!(do_parse!(
	lb: i32_digit >>
	ub: i32_digit >>
	((lb, ub))
)));

named!(parse_float_array_def<&str,Definition>, ws!(do_parse!(
	name: take_until!(" ")                             >>
	tag!("FLOAT")                                      >>
	size: i32_digit                                    >>
	dimensions: count!(parse_dimension, size as usize) >>
	(Definition {
		name: name.to_owned(),
		dimensions,
		dtype: DefinitionType::Real
	})
)));

named!(parse_int_array_def<&str,Definition>, ws!(do_parse!(
	name: take_until!(" ")                             >>
	tag!("INT")                                        >>
	size: i32_digit                                    >>
	dimensions: count!(parse_dimension, size as usize) >>
	(Definition {
		name: name.to_owned(),
		dimensions,
		dtype: DefinitionType::Integer
	})
)));

named!(parse_float_def<&str,Definition>, ws!(do_parse!(
	name: take_until!(" ")                             >>
	tag!("FLOAT")                                      >>
	(Definition {
		name: name.to_owned(),
		dimensions: Vec::new(),
		dtype: DefinitionType::Real
	})
)));

named!(parse_int_def<&str,Definition>, ws!(do_parse!(
	name: take_until!(" ")                             >>
	tag!("INT")                                        >>
	(Definition {
		name: name.to_owned(),
		dimensions: Vec::new(),
		dtype: DefinitionType::Integer
	})
)));

named!(parse_def<&str,Definition>, ws!(do_parse!(
	def: alt!(
		parse_float_array_def |
		parse_int_array_def   |
		parse_float_def       |
		parse_int_def
	)                         >>
	(def)
)));

named!(pub parse_ast<&str,Ast>, ws!(do_parse!(
	name: take_until!(" ")     >>
	vardef: many0!(parse_def)  >>
	statements: parse_stmtlist >>
	(Ast { name: name.to_owned(), vardef, statements })
)));