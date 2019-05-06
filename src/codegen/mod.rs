use std::{io, collections::{HashSet, HashMap}, marker::PhantomData};
use ::ir::*;

pub fn generate_trace<T>(ast: &Ast, out: T) -> io::Result<()>
	where T: io::Write
{
	let mut cg: Codegen<Tracer<_>,_> = Codegen::new(out);

	cg.generate_ast(ast)
}

pub struct Codegen<G, W> {
	generator: G,
	out: W
}

struct Tracer<'a, W: 'a> {
	loop_indices: HashSet<String>,
	tracer_type: PhantomData<&'a W>
}

pub struct Vectorizer<'a, W: 'a> {
	loop_replacement: HashMap<String, &'a Loop>,
	tracer_type: PhantomData<W>,
	folding: bool
}

impl<'a, W: io::Write> Tracer<'a, W> {
	fn log_use_expression(
		&self,
		out: &mut W,
		expr: &'a Expression,
		indent: u8,
		label: i32
	) -> io::Result<()>
	{
		match expr {
			Expression::Integer(_) |
			Expression::Real(_) => Ok(()),
			Expression::Expression(expr) => self.log_use_expression(out, expr, indent, label),
			Expression::Variable(var) => self.log_use_var(out, var, indent, label),
			Expression::UnOp(unop) => self.log_use_unop(out, unop, indent, label),
			Expression::BinOp(binop) => self.log_use_binop(out, binop, indent, label)
		}
	}

	fn log_use_var(
		&self,
		out: &mut W,
		var: &'a Variable,
		indent: u8,
		label: i32
	) -> io::Result<()>
	{
		if !self.loop_indices.contains(&var.name) {
			if var.indices.len() > 0 {
				write!(out,
					"    {}write (*,'(a,{}(x,i0))') ' {:03} {} USE'",
					&indentation(indent),
					var.indices.len(),
					label,
					&var.name)?;

				for expr in var.indices.iter() {
					write!(out, ", ")?;
					generate_expression(self, out, expr)?;
				}

				writeln!(out, "")?;
			} else {
				writeln!(out,
					"    {}write (*,'(a)')         ' {:03} {} USE'",
					&indentation(indent),
					label,
					&var.name)?;
			}
		}

		for expr in var.indices.iter() {
			self.log_use_expression(out, expr, indent, label)?
		}

		Ok(())
	}

	fn log_use_binop(
		&self,
		out: &mut W,
		binop: &'a BinOp,
		indent: u8,
		label: i32
	) -> io::Result<()>
	{
		self.log_use_expression(out, &binop.left, indent, label)?;
		self.log_use_expression(out, &binop.right, indent, label)
	}

	fn log_use_unop(
		&self,
		out: &mut W,
		unop: &'a UnOp,
		indent: u8,
		label: i32
	) -> io::Result<()>
	{
		self.log_use_expression(out, &unop.right, indent, label)
	}
}

pub trait Generator<'a, G, W> {
	fn instantiate(folding: bool) -> Self;

	fn log_loop_begin(
		&mut self,
		out: &mut W,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()>;

	fn log_loop_end(
		&mut self,
		out: &mut W,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()>;

	fn log_loop_update(
		&self,
		out: &mut W,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()>;

	fn log_use(
		&self,
		out: &mut W,
		expr: &'a Expression,
		indent: u8,
		label: i32
	) -> io::Result<()>;

	fn log_def(
		&self,
		out: &mut W,
		var: &'a Variable,
		indent: u8,
		label: i32
	) -> io::Result<()>;

	fn index_expression(
		&self,
		out: &mut W,
		expr: &'a Expression
	) -> io::Result<()>;

	fn set_loop_data(
		&mut self,
		loop_replacement: HashMap<String, &'a Loop>
	);
}

impl<'a, W> Vectorizer<'a, W> {
	fn check_expr(&self, expr: &'a Expression) -> bool {
		match expr {
			Expression::Variable(var) => self.loop_replacement.get(&var.name).is_some(),
			Expression::Integer(_) |
			Expression::Real(_) => false,
			Expression::UnOp(op) => self.check_expr(&op.right),
			Expression::BinOp(op) => self.check_expr(&op.left) || self.check_expr(&op.right),
			Expression::Expression(expr) => self.check_expr(expr)
		}
	}

	fn build_expr(&self, expr: &'a Expression, upper: bool, replacing: bool) -> Expression {
		match expr {
			Expression::Variable(var) => {
				if let Some(l) = self.loop_replacement.get(&var.name) {
					if upper {
						self.build_expr(&l.upper, upper, true)
					} else {
						self.build_expr(&l.lower, upper, true)
					}
				} else {
					let mut indices = Vec::new();
					for index in var.indices.iter() {
						indices.push(self.build_expr(index, upper, false));
					}

					Expression::Variable(Variable { name: var.name.to_owned(), indices })
				}
			}
			Expression::Integer(i) => Expression::Integer(*i),
			Expression::Real(r) => Expression::Real(*r),
			Expression::BinOp(op) => {
				if replacing {
					Expression::Expression(Box::new(Expression::BinOp(Box::new(BinOp {
						op: op.op.clone(),
						left: self.build_expr(&op.left, upper, false),
						right: self.build_expr(&op.right, upper, false)
					}))))
				} else {
					Expression::BinOp(Box::new(BinOp {
						op: op.op.clone(),
						left: self.build_expr(&op.left, upper, false),
						right: self.build_expr(&op.right, upper, false)
					}))
				}
			},
			Expression::UnOp(op) => {
				if replacing {
					Expression::Expression(Box::new(Expression::UnOp(Box::new(UnOp {
						op: op.op.clone(),
						right: self.build_expr(&op.right, upper, false)
					}))))
				} else {
					Expression::UnOp(Box::new(UnOp {
						op: op.op.clone(),
						right: self.build_expr(&op.right, upper, false)
					}))
				}
			}
			Expression::Expression(expr) => Expression::Expression(Box::new(self.build_expr(expr, upper, false)))
		}
	}

	fn fold_expr(&self, expr: &'a Expression) -> Expression {
		match expr {
			Expression::Variable(var) => {
				let mut indices = Vec::new();
				for index in var.indices.iter() {
					indices.push(self.fold_expr(index));
				}

				Expression::Variable(Variable { name: var.name.to_owned(), indices })
			}
			Expression::Integer(i) => Expression::Integer(*i),
			Expression::Real(r) => Expression::Real(*r),
			Expression::BinOp(op) => {
				let left = self.fold_expr(&op.left);
				let right = self.fold_expr(&op.right);

				match left {
					Expression::Integer(i_left) => {
						match right {
							Expression::Integer(i_right) => {
								match op.op {
									OpType::Plus => Expression::Integer(i_left + i_right),
									OpType::Minus => Expression::Integer(i_left - i_right),
									OpType::Mul => Expression::Integer(i_left * i_right),
									OpType::Div => Expression::Integer(i_left / i_right),
									OpType::Equal => Expression::Integer((i_left == i_right) as i32),
									OpType::NotEqual => Expression::Integer((i_left != i_right) as i32),
									OpType::Greater => Expression::Integer((i_left > i_right) as i32),
									OpType::GreaterEqual => Expression::Integer((i_left >= i_right) as i32),
									OpType::Lower => Expression::Integer((i_left < i_right) as i32),
									OpType::LowerEqual => Expression::Integer((i_left <= i_right) as i32),
									OpType::And => Expression::Integer(((i_left != 0) && (i_right != 0)) as i32),
									OpType::Or => Expression::Integer(((i_left != 0) || (i_right != 0)) as i32),
									OpType::Not => Expression::Integer((i_right == 0) as i32)
								}
							},
							Expression::Real(r_right) => {
								match op.op {
									OpType::Plus => Expression::Real(i_left as f64 + r_right),
									OpType::Minus => Expression::Real(i_left as f64 - r_right),
									OpType::Mul => Expression::Real(i_left as f64  * r_right),
									OpType::Div => Expression::Real(i_left as f64  / r_right),
									OpType::Equal => Expression::Integer((i_left as f64  == r_right) as i32),
									OpType::NotEqual => Expression::Integer((i_left as f64  != r_right) as i32),
									OpType::Greater => Expression::Integer((i_left as f64  > r_right) as i32),
									OpType::GreaterEqual => Expression::Integer((i_left as f64 >= r_right) as i32),
									OpType::Lower => Expression::Integer(((i_left as f64) < r_right) as i32),
									OpType::LowerEqual => Expression::Integer((i_left as f64  <= r_right) as i32),
									OpType::And => Expression::Integer(((i_left != 0) && (r_right != 0.0)) as i32),
									OpType::Or => Expression::Integer(((i_left != 0) || (r_right != 0.0)) as i32),
									OpType::Not => Expression::Integer((r_right == 0.0) as i32)
								}
							},
							_ => Expression::BinOp(Box::new(BinOp { op: op.op.clone(), left, right }))
						}
					},
					Expression::Real(r_left) => {
						match right {
							Expression::Integer(i_right) => {
								match op.op {
									OpType::Plus => Expression::Real(r_left + (i_right as f64)),
									OpType::Minus => Expression::Real(r_left - (i_right as f64)),
									OpType::Mul => Expression::Real(r_left * (i_right as f64)),
									OpType::Div => Expression::Real(r_left / (i_right as f64)),
									OpType::Equal => Expression::Integer((r_left == (i_right as f64)) as i32),
									OpType::NotEqual => Expression::Integer((r_left != (i_right as f64)) as i32),
									OpType::Greater => Expression::Integer((r_left > (i_right as f64)) as i32),
									OpType::GreaterEqual => Expression::Integer((r_left >= (i_right as f64)) as i32),
									OpType::Lower => Expression::Integer((r_left < (i_right as f64)) as i32),
									OpType::LowerEqual => Expression::Integer((r_left <= (i_right as f64)) as i32),
									OpType::And => Expression::Integer(((r_left != 0.0) && (i_right != 0)) as i32),
									OpType::Or => Expression::Integer(((r_left != 0.0) || (i_right != 0)) as i32),
									OpType::Not => Expression::Integer((i_right == 0) as i32)
								}
							},
							Expression::Real(r_right) => {
								match op.op {
									OpType::Plus => Expression::Real(r_left + r_right),
									OpType::Minus => Expression::Real(r_left - r_right),
									OpType::Mul => Expression::Real(r_left  * r_right),
									OpType::Div => Expression::Real(r_left  / r_right),
									OpType::Equal => Expression::Integer((r_left == r_right) as i32),
									OpType::NotEqual => Expression::Integer((r_left != r_right) as i32),
									OpType::Greater => Expression::Integer((r_left > r_right) as i32),
									OpType::GreaterEqual => Expression::Integer((r_left >= r_right) as i32),
									OpType::Lower => Expression::Integer((r_left < r_right) as i32),
									OpType::LowerEqual => Expression::Integer((r_left <= r_right) as i32),
									OpType::And => Expression::Integer(((r_left != 0.0) && (r_right != 0.0)) as i32),
									OpType::Or => Expression::Integer(((r_left != 0.0) || (r_right != 0.0)) as i32),
									OpType::Not => Expression::Integer((r_right == 0.0) as i32)
								}
							},
							_ => Expression::BinOp(Box::new(BinOp { op: op.op.clone(), left, right }))
						}
					},
					_ => Expression::BinOp(Box::new(BinOp { op: op.op.clone(), left, right }))
				}
			},
			Expression::UnOp(op) => {
				let right = self.fold_expr(&op.right);

				match right {
					Expression::Integer(i_right) => {
						match op.op {
							OpType::Plus => Expression::Integer(i_right),
							OpType::Minus => Expression::Integer(-i_right),
							OpType::Mul => Expression::Integer(i_right),
							OpType::Div => Expression::Integer(i_right),
							OpType::Equal => Expression::Integer((i_right == i_right) as i32),
							OpType::NotEqual => Expression::Integer((i_right != i_right) as i32),
							OpType::Greater => Expression::Integer((i_right > i_right) as i32),
							OpType::GreaterEqual => Expression::Integer((i_right >= i_right) as i32),
							OpType::Lower => Expression::Integer((i_right < i_right) as i32),
							OpType::LowerEqual => Expression::Integer((i_right <= i_right) as i32),
							OpType::And => Expression::Integer(((i_right != 0) && (i_right != 0)) as i32),
							OpType::Or => Expression::Integer(((i_right != 0) || (i_right != 0)) as i32),
							OpType::Not => Expression::Integer((i_right == 0) as i32)
						}
					},
					Expression::Real(r_right) => {
						match op.op {
							OpType::Plus => Expression::Real(r_right),
							OpType::Minus => Expression::Real(-r_right),
							OpType::Mul => Expression::Real(r_right),
							OpType::Div => Expression::Real(r_right),
							OpType::Equal => Expression::Integer((r_right == r_right) as i32),
							OpType::NotEqual => Expression::Integer((r_right != r_right) as i32),
							OpType::Greater => Expression::Integer((r_right > r_right) as i32),
							OpType::GreaterEqual => Expression::Integer((r_right >= r_right) as i32),
							OpType::Lower => Expression::Integer((r_right < r_right) as i32),
							OpType::LowerEqual => Expression::Integer((r_right <= r_right) as i32),
							OpType::And => Expression::Integer(((r_right != 0.0) && (r_right != 0.0)) as i32),
							OpType::Or => Expression::Integer(((r_right != 0.0) || (r_right != 0.0)) as i32),
							OpType::Not => Expression::Integer((r_right == 0.0) as i32)
						}
					},
					_ => Expression::UnOp(Box::new(UnOp { op: op.op.clone(), right }))
				}
			}
			Expression::Expression(expr) => {
				let expr = self.fold_expr(expr);
				match expr {
					Expression::Integer(_) |
					Expression::Real(_) => expr,
					_ => Expression::Expression(Box::new(expr))
				}
			}
		}
	}
}

impl<'a, W: io::Write> Generator<'a, Vectorizer<'a, W>, W> for Vectorizer<'a, W> {
	fn instantiate(folding: bool) -> Self {
		Vectorizer { loop_replacement: HashMap::new(), tracer_type: PhantomData, folding }
	}

	fn log_loop_begin(
		&mut self,
		_out: &mut W,
		_loop_node: &'a Loop,
		_indent: u8
	) -> io::Result<()> {
		Ok(())
	}

	fn log_loop_end(
		&mut self,
		_out: &mut W,
		_loop_node: &'a Loop,
		_indent: u8
	) -> io::Result<()> {
		Ok(())
	}

	fn log_loop_update(
		&self,
		_out: &mut W,
		_loop_node: &'a Loop,
		_indent: u8
	) -> io::Result<()> {
		Ok(())
	}

	fn log_use(
		&self,
		_out: &mut W,
		_expr: &'a Expression,
		_indent: u8,
		_label: i32
	) -> io::Result<()> {
		Ok(())
	}

	fn log_def(
		&self,
		_out: &mut W,
		_var: &'a Variable,
		_indent: u8,
		_label: i32
	) -> io::Result<()> {
		Ok(())
	}

	fn index_expression(
		&self,
		out: &mut W,
		expr: &'a Expression
	) -> io::Result<()> {
		if self.check_expr(expr) {
			let lower = {
				let expr = self.build_expr(expr, false, false);
				if self.folding {
					self.fold_expr(&expr)
				} else {
					expr
				}
			};
			generate_expression(self, out, &lower)?;

			write!(out, ":")?;

			let upper = {
				let expr = self.build_expr(expr, true, false);
				if self.folding {
					self.fold_expr(&expr)
				} else {
					expr
				}
			};
			generate_expression(self, out, &upper)
		} else {
			generate_expression(self, out, expr)
		}
	}

	fn set_loop_data(
		&mut self,
		loop_replacement: HashMap<String, &'a Loop>
	) {
		self.loop_replacement = loop_replacement;
	}
}


impl<'a, W: io::Write> Generator<'a, Tracer<'a, W>, W> for Tracer<'a, W> {
	fn instantiate(_folding: bool) -> Self {
		Tracer { loop_indices: HashSet::new(), tracer_type: PhantomData }
	}

	fn log_loop_begin(
		&mut self,
		out: &mut W,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()>
	{
		self.loop_indices.insert(loop_node.var.to_owned());

		writeln!(out,
			"    {1:}write (*,'(a)')         ' {0:} {2:} loop begin'",
			loop_node.label,
			&indentation(indent),
			&loop_node.var)
	}

	fn log_loop_end(
		&mut self,
		out: &mut W,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()>
	{
		self.loop_indices.remove(&loop_node.var);

		writeln!(out,
			"    {1:}write (*,'(a)')         ' {0:} {2:} loop end'",
			loop_node.label,
			&indentation(indent),
			&loop_node.var)
	}

	fn log_loop_update(
		&self,
		out: &mut W,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()> {
		writeln!(out,
			"    {1:}write (*,'(a,i0)')      ' {0:} {2:} ', {2:}",
			loop_node.label,
			&indentation(indent),
			&loop_node.var)
	}

	fn log_use(
		&self,
		out: &mut W,
		expr: &'a Expression,
		indent: u8,
		label: i32
	) -> io::Result<()>
	{
		self.log_use_expression(out, expr, indent, label)
	}

	fn log_def(
		&self,
		out: &mut W,
		var: &'a Variable,
		indent: u8,
		label: i32
	) -> io::Result<()>
	{
		if var.indices.len() > 0 {
			write!(out,
				"    {}write (*,'(a,{}(x,i0))') ' {:03} {} DEF'",
				&indentation(indent),
				var.indices.len(),
				label,
				&var.name)?;

			for expr in var.indices.iter() {
				write!(out, ", ")?;
				generate_expression(self, out, expr)?;
			}

			writeln!(out, "")?;
		} else {
			writeln!(out,
				"    {}write (*,'(a)')         ' {:03} {} DEF'",
				&indentation(indent),
				label,
				&var.name)?;
		}

		for expr in var.indices.iter() {
			self.log_use_expression(out, expr, indent, label)?;
		}

		Ok(())
	}

	fn index_expression(
		&self,
		out: &mut W,
		expr: &'a Expression
	) -> io::Result<()> {
		generate_expression(self, out, expr)
	}

	fn set_loop_data(
		&mut self,
		_loop_replacement: HashMap<String, &'a Loop>
	) {

	}
}

impl<'a, G: Generator<'a, G, W>, W: io::Write> Codegen<G, W> {
	pub fn new(out: W) -> Self {
		Codegen { generator: G::instantiate(false), out }
	}

	pub fn new_folding(out: W) -> Self {
		Codegen { generator: G::instantiate(true), out }
	}

	pub fn set_loop_data(&mut self, loop_replacement: HashMap<String, &'a Loop>) {
		self.generator.set_loop_data(loop_replacement);
	}

	pub fn generate_ast(&mut self, ast: &'a Ast) -> io::Result<()> {
		// Generate header
		self.generate_header(ast)?;

		// Generate statements
		self.generate_stmtlist(&ast.statements.0, 0)?;

		// Generate footer
		self.generate_footer(ast)
	}

	pub fn generate_header(&mut self, ast: &'a Ast) -> io::Result<()> {
		writeln!(self.out,
			"! Compilers for Parallel Systems\n\
			 ! 185.A64 SS 2018 H. Moritsch\n\
			 ! F90 generated from EFL source\n"
		)?;

		writeln!(self.out, "program {}\n", &ast.name)?;

		// Generate definitions
		for def in ast.vardef.iter() {
			self.generate_definition(def)?;
		}
		writeln!(self.out, "")
	}

	pub fn generate_footer(&mut self, ast: &'a Ast) -> io::Result<()> {
		writeln!(self.out, "")?;
		writeln!(self.out, "end program {}", &ast.name)
	}

	pub fn generate_loop_vec_start(
		&mut self,
		loop_node: &'a Loop,
		c: i32,
	) -> io::Result<()> {
		self.generator.log_loop_begin(&mut self.out, &loop_node, c as u8)?;

		write!(self.out,
			"    {}do {} = ",
			&indentation(c as u8),
			&loop_node.var)?;
		generate_expression(&self.generator, &mut self.out, &loop_node.lower)?;
		write!(self.out, ", ")?;
		generate_expression(&self.generator, &mut self.out, &loop_node.upper)?;
		writeln!(self.out, "")?;

		self.generator.log_loop_update(&mut self.out, &loop_node, c as u8 + 1)
	}

	pub fn generate_loop_vec_end(
		&mut self,
		loop_node: &'a Loop,
		c: i32,
	) -> io::Result<()> {
		writeln!(&mut self.out,
			"    {}end do",
			&indentation(c as u8))?;

		self.generator.log_loop_end(&mut self.out, &loop_node, c as u8)
	}

	fn generate_definition(&mut self, def: &'a Definition) -> io::Result<()> {
		match def.dtype {
			DefinitionType::Integer => write!(self.out, "integer")?,
			DefinitionType::Real => write!(self.out, "real")?
		};

		let mut iter = def.dimensions.iter();
		if let Some((lb,ub)) = iter.next() {
			write!(self.out, ", dimension({}:{}", lb, ub)?;

			while let Some((lb,ub)) = iter.next() {
				write!(self.out, ",{}:{}", lb, ub)?;
			}

			write!(self.out, ")")?;
		}


		writeln!(self.out, " :: {}", &def.name)
	}

	fn generate_stmtlist(
		&mut self,
		stmtlist: &'a [Statement],
		indent: u8
	) -> io::Result<()> {
		for statement in stmtlist {
			self.generate_statement(statement, indent)?;
		}

		Ok(())
	}

	fn generate_statement(
		&mut self,
		statement: &'a Statement,
		indent: u8
	) -> io::Result<()> {
		match statement {
			Statement::Loop(l) => self.generate_loop(l, indent),
			Statement::Assignment(a) => self.generate_assignment(a, indent),
			Statement::If(i) => self.generate_if(i, indent)
		}
	}

	fn generate_loop(
		&mut self,
		loop_node: &'a Loop,
		indent: u8
	) -> io::Result<()> {
		self.generator.log_loop_begin(&mut self.out, &loop_node, indent)?;

		write!(self.out,
			"{:03} {}do {} = ",
			loop_node.label,
			&indentation(indent),
			&loop_node.var)?;

		generate_expression(&self.generator, &mut self.out, &loop_node.lower)?;
		write!(self.out, ", ")?;
		generate_expression(&self.generator, &mut self.out, &loop_node.upper)?;
		writeln!(self.out, "")?;

		self.generator.log_loop_update(&mut self.out, &loop_node, indent + 1)?;
		self.generate_stmtlist(&loop_node.statements.0, indent + 1)?;

		writeln!(self.out, "    {}end do", &indentation(indent))?;
		self.generator.log_loop_end(&mut self.out, &loop_node, indent)
	}

	pub fn generate_assignment(
		&mut self,
		assignment: &'a Assign,
		indent: u8
	) -> io::Result<()> {
		self.generator.log_def(&mut self.out, &assignment.lhs, indent, assignment.label)?;
		self.generator.log_use(&mut self.out, &assignment.rhs, indent, assignment.label)?;

		write!(self.out,
			"{:03} {}",
			assignment.label,
			&indentation(indent))?;

		generate_variable(&self.generator, &mut self.out, &assignment.lhs)?;
		write!(self.out, " = ")?;
		generate_expression(&self.generator, &mut self.out, &assignment.rhs)?;
		writeln!(self.out, "")
	}

	fn generate_if(
		&mut self,
		if_stat: &'a If,
		indent: u8
	) -> io::Result<()> {
		write!(self.out, "{:03} {}if (", if_stat.label, &indentation(indent))?;
		generate_expression(&self.generator, &mut self.out, &if_stat.expr)?;
		writeln!(self.out, ") then")?;

		self.generate_stmtlist(&if_stat.then_branch.0, indent + 1)?;
		writeln!(self.out, "    {}else", &indentation(indent))?;
		self.generate_stmtlist(&if_stat.else_branch.0, indent + 1)?;

		writeln!(self.out, "    {}end if", &indentation(indent))
	}
}

fn generate_variable<'a, W, G>(
	gen: &G,
	out: &mut W,
	var: &'a Variable
) -> io::Result<()>
	where W: io::Write, G: Generator<'a, G, W>
{
	write!(out, "{}", &var.name)?;

	let mut iter = var.indices.iter();
	if let Some(expr) = iter.next() {
		write!(out, "(")?;
		gen.index_expression(out, expr)?;

		while let Some(expr) = iter.next() {
			write!(out, ",")?;
			gen.index_expression(out, expr)?;
		}

		write!(out, ")")?;
	}

	Ok(())
}

pub fn generate_expression<'a, G, W>(
	gen: &G,
	out: &mut W,
	expr: &'a Expression
) -> io::Result<()>
	where W: io::Write, G: Generator<'a, G, W>
{
	match expr {
		Expression::Integer(i) => write!(out, "{}", i),
		Expression::Real(f) => write!(out, "{}", f),
		Expression::Variable(var) => generate_variable(gen, out, var),
		Expression::BinOp(op) => generate_binop(gen, out, op),
		Expression::UnOp(op) => generate_unop(gen, out, op),
		Expression::Expression(expr) => {
			write!(out, "(")?;
			generate_expression(gen, out, expr)?;
			write!(out, ")")
		}
	}
}

fn generate_unop<'a, G, W>(
	gen: &G,
	out: &mut W,
	op: &'a UnOp
) -> io::Result<()>
	where W: io::Write, G: Generator<'a, G, W>
{
	match op.op {
		OpType::Plus => write!(out, "+")?,
		OpType::Minus => write!(out, "-")?,
		OpType::Mul => write!(out, "*")?,
		OpType::Div => write!(out, "/")?,
		OpType::Equal => write!(out, " == ")?,
		OpType::NotEqual => write!(out, " /= ")?,
		OpType::Greater => write!(out, " > ")?,
		OpType::GreaterEqual => write!(out, " >= ")?,
		OpType::Lower => write!(out, " < ")?,
		OpType::LowerEqual => write!(out, " <= ")?,
		OpType::And => write!(out, " .and. ")?,
		OpType::Or => write!(out, " .or. ")?,
		OpType::Not => write!(out, " .not. ")?
	}

	generate_expression(gen, out, &op.right)
}

fn generate_binop<'a, G, W>(
	gen: &G,
	out: &mut W,
	op: &'a BinOp
) -> io::Result<()>
	where W: io::Write, G: Generator<'a, G, W>
{
	generate_expression(gen, out, &op.left)?;

	match op.op {
		OpType::Plus => write!(out, "+")?,
		OpType::Minus => write!(out, "-")?,
		OpType::Mul => write!(out, "*")?,
		OpType::Div => write!(out, "/")?,
		OpType::Equal => write!(out, " == ")?,
		OpType::NotEqual => write!(out, " /= ")?,
		OpType::Greater => write!(out, " > ")?,
		OpType::GreaterEqual => write!(out, " >= ")?,
		OpType::Lower => write!(out, " < ")?,
		OpType::LowerEqual => write!(out, " <= ")?,
		OpType::And => write!(out, " .and. ")?,
		OpType::Or => write!(out, " .or. ")?,
		OpType::Not => write!(out, " .not. ")?
	}

	generate_expression(gen, out, &op.right)
}

pub fn indentation(indent: u8) -> String {
	" ".repeat(4 * indent as usize)
}