use super::*;
use nom::{digit, double_s};
use std::str::FromStr;

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
        parse_unop     |
        parse_var_expr |
        map!(parse_expr, |e| Expression::Expression(Box::new(e)))) >>
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

named!(parse_binoptype<&str,OpType>, ws!(alt!(
    map!(tag!("+"), |_| OpType::Plus) |
    map!(tag!("-"), |_| OpType::Minus) |
    map!(tag!("*"), |_| OpType::Mul) |
    map!(tag!("/"), |_| OpType::Div) |
    map!(tag!("=="), |_| OpType::Equal) |
    map!(tag!("<>"), |_| OpType::NotEqual) |
    map!(tag!(">"), |_| OpType::Greater) |
    map!(tag!(">="), |_| OpType::GreaterEqual) |
    map!(tag!("<"), |_| OpType::Lower) |
    map!(tag!("<="), |_| OpType::LowerEqual) |
    map!(tag!(".and."), |_| OpType::And) |
    map!(tag!(".or."), |_| OpType::Or) |
    map!(tag!(".not."), |_| OpType::Not)
)));

named!(parse_unop<&str,Expression>, ws!(do_parse!(
    tag!("UNOP")       >>
    op: parse_binoptype >>
    right: parse_expr   >>
    (Expression::UnOp(Box::new(UnOp { op, right })))
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

named!(parse_if<&str,Statement>, ws!(do_parse!(
    tag!("FOR")                       >>
    tag!("@")                         >>
    label: i32_digit                  >>
    expr: parse_expr                  >>
    then_branch: parse_stmtlist       >>
    else_branch: parse_stmtlist       >>
    (Statement::If(If {
        label, expr, then_branch, else_branch
    }))
)));

named!(parse_loop<&str,Statement>, ws!(do_parse!(
    tag!("FOR")                       >>
    tag!("@")                         >>
    label: i32_digit                  >>
    var: parse_entry                  >>
    lower: parse_expr                 >>
    upper: parse_expr                 >>
    statements: parse_stmtlist        >>
    (Statement::Loop(Loop {
        label, var, lower, upper,
        statements
    }))
)));

named!(parse_stmtlist<&str,StatementList>, ws!(do_parse!(
    tag!("STMTLIST")                                         >>
    list: many0!(alt!(parse_assign | parse_loop | parse_if)) >>
    tag!("/STMTLIST")                                        >>
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
