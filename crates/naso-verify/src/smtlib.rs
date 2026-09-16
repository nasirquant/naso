//! SMT-LIB2 AST and pretty-printer.

use std::fmt::{self, Write};

/// SMT-LIB2 script representation.
#[derive(Debug, Clone)]
pub struct Script {
    pub commands: Vec<Command>,
}

impl Script {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn set_logic(&mut self, logic: &str) {
        self.commands.push(Command::SetLogic(logic.to_string()));
    }

    pub fn declare_sort(&mut self, name: &str, arity: usize) {
        self.commands
            .push(Command::DeclareSort(name.to_string(), arity));
    }

    pub fn define_sort(&mut self, name: &str, params: Vec<String>, def: Sort) {
        self.commands
            .push(Command::DefineSort(name.to_string(), params, def));
    }

    pub fn declare_fun(&mut self, name: &str, args: Vec<Sort>, ret: Sort) {
        self.commands
            .push(Command::DeclareFun(name.to_string(), args, ret));
    }

    pub fn declare_const(&mut self, name: &str, sort: Sort) {
        self.commands
            .push(Command::DeclareConst(name.to_string(), sort));
    }

    pub fn assert(&mut self, term: Term) {
        self.commands.push(Command::Assert(term));
    }

    pub fn push(&mut self, n: u32) {
        self.commands.push(Command::Push(n));
    }

    pub fn pop(&mut self, n: u32) {
        self.commands.push(Command::Pop(n));
    }

    pub fn check_sat(&mut self) {
        self.commands.push(Command::CheckSat);
    }

    pub fn check_sat_assuming(&mut self, assumptions: Vec<Term>) {
        self.commands.push(Command::CheckSatAssuming(assumptions));
    }

    pub fn get_model(&mut self) {
        self.commands.push(Command::GetModel);
    }

    pub fn get_unsat_core(&mut self) {
        self.commands.push(Command::GetUnsatCore);
    }

    pub fn get_value(&mut self, terms: Vec<Term>) {
        self.commands.push(Command::GetValue(terms));
    }

    pub fn exit(&mut self) {
        self.commands.push(Command::Exit);
    }

    /// Emit as SMT-LIB2 string.
    pub fn to_string(&self) -> String {
        let mut out = String::new();
        for cmd in &self.commands {
            cmd.fmt(&mut out).unwrap();
            out.push('\n');
        }
        out
    }
}

impl Default for Script {
    fn default() -> Self {
        Self::new()
    }
}

/// SMT-LIB2 commands.
#[derive(Debug, Clone)]
pub enum Command {
    SetLogic(String),
    DeclareSort(String, usize),
    DefineSort(String, Vec<String>, Sort),
    DeclareFun(String, Vec<Sort>, Sort),
    DeclareConst(String, Sort),
    Assert(Term),
    Push(u32),
    Pop(u32),
    CheckSat,
    CheckSatAssuming(Vec<Term>),
    GetModel,
    GetUnsatCore,
    GetValue(Vec<Term>),
    Exit,
    Comment(String),
    SetOption(String, AttributeValue),
    SetInfo(AttributeValue),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::SetLogic(logic) => write!(f, "(set-logic {})", logic),
            Command::DeclareSort(name, arity) => write!(f, "(declare-sort {} {})", name, arity),
            Command::DefineSort(name, params, def) => {
                write!(f, "(define-sort {} (", name)?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") {})", def)
            }
            Command::DeclareFun(name, args, ret) => {
                write!(f, "(declare-fun {} (", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ") {})", ret)
            }
            Command::DeclareConst(name, sort) => write!(f, "(declare-const {} {})", name, sort),
            Command::Assert(term) => write!(f, "(assert {})", term),
            Command::Push(n) => write!(f, "(push {})", n),
            Command::Pop(n) => write!(f, "(pop {})", n),
            Command::CheckSat => write!(f, "(check-sat)"),
            Command::CheckSatAssuming(assumptions) => {
                write!(f, "(check-sat-assuming")?;
                for a in assumptions {
                    write!(f, " {}", a)?;
                }
                write!(f, ")")
            }
            Command::GetModel => write!(f, "(get-model)"),
            Command::GetUnsatCore => write!(f, "(get-unsat-core)"),
            Command::GetValue(terms) => {
                write!(f, "(get-value")?;
                for t in terms {
                    write!(f, " {}", t)?;
                }
                write!(f, ")")
            }
            Command::Exit => write!(f, "(exit)"),
            Command::Comment(s) => write!(f, "; {}", s),
            Command::SetOption(opt, val) => write!(f, "(set-option :{} {})", opt, val),
            Command::SetInfo(val) => write!(f, "(set-info {})", val),
        }
    }
}

/// SMT-LIB2 sorts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sort {
    Bool,
    Int,
    Real,
    BitVec(u32),
    Array(Box<Sort>, Box<Sort>),
    Datatype(String),
    Function(Vec<Sort>, Box<Sort>),
    Custom(String),
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sort::Bool => write!(f, "Bool"),
            Sort::Int => write!(f, "Int"),
            Sort::Real => write!(f, "Real"),
            Sort::BitVec(n) => write!(f, "(_ BitVec {})", n),
            Sort::Array(idx, elem) => write!(f, "(Array {} {})", idx, elem),
            Sort::Datatype(name) => write!(f, "{}", name),
            Sort::Function(args, ret) => {
                write!(f, "(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ") {}", ret)
            }
            Sort::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// SMT-LIB2 terms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Const(Constant),
    Var(String, Sort),
    App(String, Vec<Term>),
    Let(Vec<(String, Term)>, Box<Term>),
    Forall(Vec<(String, Sort)>, Box<Term>),
    Exists(Vec<(String, Sort)>, Box<Term>),
    Match(Box<Term>, Vec<MatchCase>),
    Annotated(Box<Term>, Vec<Attribute>),
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Const(c) => write!(f, "{}", c),
            Term::Var(name, sort) => write!(f, "{}", name), // Sort in declaration
            Term::App(name, args) => {
                write!(f, "({}", name)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            Term::Let(bindings, body) => {
                write!(f, "(let (")?;
                for (i, (name, val)) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({} {})", name, val)?;
                }
                write!(f, ") {})", body)
            }
            Term::Forall(vars, body) => {
                write!(f, "(forall (")?;
                for (i, (name, sort)) in vars.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({} {})", name, sort)?;
                }
                write!(f, ") {})", body)
            }
            Term::Exists(vars, body) => {
                write!(f, "(exists (")?;
                for (i, (name, sort)) in vars.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({} {})", name, sort)?;
                }
                write!(f, ") {})", body)
            }
            Term::Match(scrutinee, cases) => {
                write!(f, "(match {} ", scrutinee)?;
                for case in cases {
                    write!(f, " {}", case)?;
                }
                write!(f, ")")
            }
            Term::Annotated(term, attrs) => {
                write!(f, "(! {} ", term)?;
                for attr in attrs {
                    write!(f, " {}", attr)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// SMT-LIB2 constants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant {
    Bool(bool),
    Int(i64),
    Real(String),     // Decimal string
    BitVec(u32, u64), // (width, value)
    String(String),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Constant::Int(i) => write!(f, "{}", i),
            Constant::Real(s) => write!(f, "{}", s),
            Constant::BitVec(w, v) => write!(f, "(_ bv{} {})", v, w),
            Constant::String(s) => write!(f, "\"{}\"", s.replace('"', "\\\"")),
        }
    }
}

/// Match case for pattern matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Term,
}

impl fmt::Display for MatchCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.pattern, self.body)
    }
}

/// Patterns for match expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    Var(String),
    Constructor(String, Vec<Pattern>),
    Const(Constant),
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Wildcard => write!(f, "_"),
            Pattern::Var(name) => write!(f, "{}", name),
            Pattern::Constructor(name, args) => {
                write!(f, "({}", name)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            Pattern::Const(c) => write!(f, "{}", c),
        }
    }
}

/// SMT-LIB2 attributes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Attribute {
    Keyword(String),
    KeywordValue(String, AttributeValue),
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Attribute::Keyword(k) => write!(f, ":{}", k),
            Attribute::KeywordValue(k, v) => write!(f, ":{} {}", k, v),
        }
    }
}

/// Attribute values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeValue {
    Symbol(String),
    String(String),
    Number(i64),
    List(Vec<AttributeValue>),
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttributeValue::Symbol(s) => write!(f, "{}", s),
            AttributeValue::String(s) => write!(f, "\"{}\"", s.replace('"', "\\\"")),
            AttributeValue::Number(n) => write!(f, "{}", n),
            AttributeValue::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Builder for constructing terms ergonomically.
pub mod builder {
    use super::*;

    pub fn bool(b: bool) -> Term {
        Term::Const(Constant::Bool(b))
    }

    pub fn int(i: i64) -> Term {
        Term::Const(Constant::Int(i))
    }

    pub fn bv(width: u32, value: u64) -> Term {
        Term::Const(Constant::BitVec(width, value))
    }

    pub fn var(name: &str, sort: Sort) -> Term {
        Term::Var(name.to_string(), sort)
    }

    pub fn app(name: &str, args: Vec<Term>) -> Term {
        Term::App(name.to_string(), args)
    }

    pub fn not(t: Term) -> Term {
        app("not", vec![t])
    }

    pub fn and(terms: Vec<Term>) -> Term {
        app("and", terms)
    }

    pub fn or(terms: Vec<Term>) -> Term {
        app("or", terms)
    }

    pub fn eq(lhs: Term, rhs: Term) -> Term {
        app("=", vec![lhs, rhs])
    }

    pub fn distinct(terms: Vec<Term>) -> Term {
        app("distinct", terms)
    }

    pub fn implies(lhs: Term, rhs: Term) -> Term {
        app("=>", vec![lhs, rhs])
    }

    pub fn ite(cond: Term, then_t: Term, else_t: Term) -> Term {
        app("ite", vec![cond, then_t, else_t])
    }

    pub fn forall(vars: Vec<(String, Sort)>, body: Term) -> Term {
        Term::Forall(vars, Box::new(body))
    }

    pub fn exists(vars: Vec<(String, Sort)>, body: Term) -> Term {
        Term::Exists(vars, Box::new(body))
    }

    pub fn add(args: Vec<Term>) -> Term {
        app("+", args)
    }

    pub fn sub(args: Vec<Term>) -> Term {
        app("-", args)
    }

    pub fn mul(args: Vec<Term>) -> Term {
        app("*", args)
    }

    pub fn div(lhs: Term, rhs: Term) -> Term {
        app("div", vec![lhs, rhs])
    }

    pub fn le(lhs: Term, rhs: Term) -> Term {
        app("<=", vec![lhs, rhs])
    }

    pub fn lt(lhs: Term, rhs: Term) -> Term {
        app("<", vec![lhs, rhs])
    }

    pub fn ge(lhs: Term, rhs: Term) -> Term {
        app(">=", vec![lhs, rhs])
    }

    pub fn gt(lhs: Term, rhs: Term) -> Term {
        app(">", vec![lhs, rhs])
    }

    pub fn select(array: Term, index: Term) -> Term {
        app("select", vec![array, index])
    }

    pub fn store(array: Term, index: Term, value: Term) -> Term {
        app("store", vec![array, index, value])
    }
}

/// Common SMT-LIB2 theory symbols.
pub mod theory {
    pub const BOOL: &str = "Bool";
    pub const INT: &str = "Int";
    pub const REAL: &str = "Real";
    pub fn BV(w: u32) -> String {
        format!("(_ BitVec {})", w)
    }
    pub fn ARRAY(idx: &str, elem: &str) -> String {
        format!("(Array {} {})", idx, elem)
    }
}
