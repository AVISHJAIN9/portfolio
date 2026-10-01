//! SQL Lexer, Parser, AST and Optimizer (Features 04-021, 04-022, 04-023, 04-025, 04-027, 04-028, 04-040, 04-041)

use crate::execution::aggregate::AggOp;
use crate::execution::vector::Predicate;
use crate::optimizer::plan::QueryPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Select,
    From,
    Where,
    GroupBy,
    Join,
    On,
    And,
    Eq,
    Gt,
    Lt,
    Dot,
    Ident(String),
    Number(i64),
    StringLit(String),
    Comma,
    Star,
    Sum,
    Count,
    OpenParen,
    CloseParen,
}

pub struct SqlLexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> SqlLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, &'static str> {
        let mut tokens = Vec::new();
        let bytes = self.src.as_bytes();

        while self.pos < bytes.len() {
            let c = bytes[self.pos];
            if c.is_ascii_whitespace() {
                self.pos += 1;
                continue;
            }

            match c {
                b',' => { tokens.push(Token::Comma); self.pos += 1; }
                b'*' => { tokens.push(Token::Star); self.pos += 1; }
                b'.' => { tokens.push(Token::Dot); self.pos += 1; }
                b'(' => { tokens.push(Token::OpenParen); self.pos += 1; }
                b')' => { tokens.push(Token::CloseParen); self.pos += 1; }
                b'=' => { tokens.push(Token::Eq); self.pos += 1; }
                b'>' => { tokens.push(Token::Gt); self.pos += 1; }
                b'<' => { tokens.push(Token::Lt); self.pos += 1; }
                b'\'' => {
                    self.pos += 1;
                    let start = self.pos;
                    while self.pos < bytes.len() && bytes[self.pos] != b'\'' {
                        self.pos += 1;
                    }
                    if self.pos >= bytes.len() {
                        return Err("Unterminated string literal");
                    }
                    let s = &self.src[start..self.pos];
                    self.pos += 1;
                    tokens.push(Token::StringLit(s.to_string()));
                }
                _ if c.is_ascii_digit() || (c == b'-' && self.pos + 1 < bytes.len() && bytes[self.pos + 1].is_ascii_digit()) => {
                    let start = self.pos;
                    if c == b'-' { self.pos += 1; }
                    while self.pos < bytes.len() && bytes[self.pos].is_ascii_digit() {
                        self.pos += 1;
                    }
                    let num: i64 = self.src[start..self.pos].parse().map_err(|_| "Invalid number")?;
                    tokens.push(Token::Number(num));
                }
                _ if c.is_ascii_alphabetic() || c == b'_' => {
                    let start = self.pos;
                    while self.pos < bytes.len() && (bytes[self.pos].is_ascii_alphanumeric() || bytes[self.pos] == b'_' || bytes[self.pos] == b'.') {
                        self.pos += 1;
                    }
                    let word = &self.src[start..self.pos];
                    let tok = match word.to_uppercase().as_str() {
                        "SELECT" => Token::Select,
                        "FROM" => Token::From,
                        "WHERE" => Token::Where,
                        "GROUP" => {
                            tokens.push(Token::GroupBy);
                            continue;
                        }
                        "BY" => continue,
                        "JOIN" => Token::Join,
                        "ON" => Token::On,
                        "AND" => Token::And,
                        "SUM" => Token::Sum,
                        "COUNT" => Token::Count,
                        _ => Token::Ident(word.to_string()),
                    };
                    tokens.push(tok);
                }
                _ => return Err("Unexpected character"),
            }
        }
        Ok(tokens)
    }
}

#[derive(Debug, Clone)]
pub struct SqlAst {
    pub select_items: Vec<String>,
    pub from_table: String,
    pub join_table: Option<(String, String, String)>, // (table, left_col, right_col)
    pub where_clauses: Vec<Predicate>,
    pub group_by: Option<String>,
    pub aggregate: Option<AggOp>,
}

pub struct SqlParser;

impl SqlParser {
    pub fn parse(sql: &str) -> Result<SqlAst, &'static str> {
        let mut lexer = SqlLexer::new(sql);
        let tokens = lexer.tokenize()?;
        let mut idx = 0;

        if tokens.is_empty() || tokens[idx] != Token::Select {
            return Err("Query must begin with SELECT");
        }
        idx += 1;

        let mut select_items = Vec::new();
        let mut aggregate = None;

        while idx < tokens.len() && tokens[idx] != Token::From {
            match &tokens[idx] {
                Token::Star => {
                    select_items.push("*".to_string());
                    idx += 1;
                }
                Token::Sum => {
                    idx += 1;
                    if idx < tokens.len() && tokens[idx] == Token::OpenParen { idx += 1; }
                    if let Token::Ident(col) = &tokens[idx] {
                        aggregate = Some(AggOp::Sum(col.clone()));
                        select_items.push(format!("SUM({})", col));
                        idx += 1;
                    }
                    if idx < tokens.len() && tokens[idx] == Token::CloseParen { idx += 1; }
                }
                Token::Count => {
                    idx += 1;
                    if idx < tokens.len() && tokens[idx] == Token::OpenParen { idx += 1; }
                    if idx < tokens.len() && tokens[idx] == Token::Star { idx += 1; }
                    if idx < tokens.len() && tokens[idx] == Token::CloseParen { idx += 1; }
                    aggregate = Some(AggOp::Count);
                    select_items.push("COUNT(*)".to_string());
                }
                Token::Ident(col) => {
                    select_items.push(col.clone());
                    idx += 1;
                }
                Token::Comma => { idx += 1; }
                _ => break,
            }
        }

        if idx >= tokens.len() || tokens[idx] != Token::From {
            return Err("Expected FROM");
        }
        idx += 1;

        let from_table = match &tokens[idx] {
            Token::Ident(tbl) => tbl.clone(),
            _ => return Err("Expected table name after FROM"),
        };
        idx += 1;

        let mut join_table = None;
        if idx < tokens.len() && tokens[idx] == Token::Join {
            idx += 1;
            let j_tbl = match &tokens[idx] {
                Token::Ident(t) => t.clone(),
                _ => return Err("Expected table name after JOIN"),
            };
            idx += 1;
            if idx < tokens.len() && tokens[idx] == Token::On {
                idx += 1;
                let left_col = match &tokens[idx] { Token::Ident(c) => c.clone(), _ => return Err("Expected left ON column") };
                idx += 1;
                if idx < tokens.len() && tokens[idx] == Token::Eq { idx += 1; }
                let right_col = match &tokens[idx] { Token::Ident(c) => c.clone(), _ => return Err("Expected right ON column") };
                idx += 1;
                join_table = Some((j_tbl, left_col, right_col));
            }
        }

        let mut where_clauses = Vec::new();
        if idx < tokens.len() && tokens[idx] == Token::Where {
            idx += 1;
            while idx < tokens.len() && tokens[idx] != Token::GroupBy {
                if let Token::Ident(col) = &tokens[idx] {
                    idx += 1;
                    let op = &tokens[idx];
                    idx += 1;
                    match op {
                        Token::Gt => {
                            if let Token::Number(num) = &tokens[idx] {
                                where_clauses.push(Predicate::Gt(col.clone(), *num));
                                idx += 1;
                            }
                        }
                        Token::Lt => {
                            if let Token::Number(num) = &tokens[idx] {
                                where_clauses.push(Predicate::Lt(col.clone(), *num));
                                idx += 1;
                            }
                        }
                        Token::Eq => {
                            if let Token::Number(num) = &tokens[idx] {
                                where_clauses.push(Predicate::Eq(col.clone(), *num));
                                idx += 1;
                            } else if let Token::StringLit(s) = &tokens[idx] {
                                where_clauses.push(Predicate::StrEq(col.clone(), s.clone()));
                                idx += 1;
                            }
                        }
                        _ => {}
                    }
                }
                if idx < tokens.len() && tokens[idx] == Token::And {
                    idx += 1;
                } else {
                    break;
                }
            }
        }

        let mut group_by = None;
        if idx < tokens.len() && tokens[idx] == Token::GroupBy {
            idx += 1;
            if let Token::Ident(g) = &tokens[idx] {
                group_by = Some(g.clone());
            }
        }

        Ok(SqlAst {
            select_items,
            from_table,
            join_table,
            where_clauses,
            group_by,
            aggregate,
        })
    }
}

pub struct LogicalOptimizer;

impl LogicalOptimizer {
    pub fn optimize(ast: &SqlAst) -> QueryPlan {
        let mut predicates = ast.where_clauses.clone();
        // Predicate pushdown rule: equality filters first (selectivity rank 0), then range (rank 1)
        predicates.sort_by_key(|p| match p {
            Predicate::Eq(_, _) | Predicate::StrEq(_, _) => 0,
            Predicate::Gt(_, _) | Predicate::Lt(_, _) => 1,
            _ => 2,
        });

        // Projection pruning rule
        let mut projections = ast.select_items.clone();
        if projections.is_empty() {
            projections.push("*".to_string());
        }

        QueryPlan {
            table_name: ast.from_table.clone(),
            projections,
            predicates,
            aggregate: ast.aggregate.clone(),
            group_by_col: ast.group_by.clone(),
        }
    }
}
