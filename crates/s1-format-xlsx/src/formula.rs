//! Formula engine — tokenizer, parser, evaluator, and dependency graph.
//!
//! Supports spreadsheet formulas like `=SUM(A1:A10)`, `=IF(A1>5,"big","small")`.
//!
//! ## Architecture
//!
//! ```text
//! Formula string  →  Tokenizer  →  Parser (AST)  →  Evaluator  →  CellValue
//!                                                       ↑
//!                                               CellLookup trait
//! ```

use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::{CellError, CellRange, CellRef, CellValue};

// ─── Token types ───────────────────────────────────────────

/// Operator tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Percent,
}

/// A single formula token produced by the tokenizer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    String(String),
    Bool(bool),
    CellRef(FormulaRef),
    RangeRef(FormulaRange),
    SheetRef(String, Box<Token>),
    Function(String),
    Operator(Op),
    OpenParen,
    CloseParen,
    Comma,
    Error(CellError),
}

/// A cell reference that tracks absolute/relative per axis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormulaRef {
    pub col: u32,
    pub row: u32,
    pub abs_col: bool,
    pub abs_row: bool,
}

impl FormulaRef {
    pub fn new(col: u32, row: u32, abs_col: bool, abs_row: bool) -> Self {
        Self {
            col,
            row,
            abs_col,
            abs_row,
        }
    }

    /// Convert to the model's plain CellRef.
    pub fn to_cell_ref(&self) -> CellRef {
        CellRef::new(self.col, self.row)
    }
}

/// A range reference for formulas (may carry absolute flags).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaRange {
    pub start: FormulaRef,
    pub end: FormulaRef,
}

impl FormulaRange {
    /// Convert to the model's plain CellRange.
    pub fn to_cell_range(&self) -> CellRange {
        CellRange::new(self.start.to_cell_ref(), self.end.to_cell_ref())
    }
}

// ─── AST ───────────────────────────────────────────────────

/// Expression node in the formula AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(CellValue),
    CellRef(FormulaRef),
    Range(FormulaRange),
    SheetRef(String, Box<Expr>),
    BinaryOp(Op, Box<Expr>, Box<Expr>),
    UnaryOp(Op, Box<Expr>),
    FunctionCall(String, Vec<Expr>),
    Percent(Box<Expr>),
}

// ─── Tokenizer ─────────────────────────────────────────────

/// Tokenize a formula string into a sequence of tokens.
///
/// The leading `=` should already be stripped before calling this function.
///
/// # Errors
///
/// Returns an error string if the formula contains unexpected characters.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Whitespace — skip
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // String literal: "..."
        if ch == '"' {
            i += 1;
            let mut s = String::new();
            while i < len {
                if chars[i] == '"' {
                    // Doubled quote inside string
                    if i + 1 < len && chars[i + 1] == '"' {
                        s.push('"');
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            tokens.push(Token::String(s));
            continue;
        }

        // Error literals: #DIV/0!, #VALUE!, #REF!, #NAME?, #NUM!, #N/A, #NULL!
        if ch == '#' {
            let start = i;
            i += 1;
            while i < len
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '/'
                    || chars[i] == '!'
                    || chars[i] == '?')
            {
                i += 1;
            }
            let err_str = &input[start..i];
            let err = match err_str {
                "#DIV/0!" => CellError::DivZero,
                "#VALUE!" => CellError::Value,
                "#REF!" => CellError::Ref,
                "#NAME?" => CellError::Name,
                "#NUM!" => CellError::Num,
                "#N/A" => CellError::NA,
                "#NULL!" => CellError::Null,
                _ => return Err(format!("Unknown error literal: {err_str}")),
            };
            tokens.push(Token::Error(err));
            continue;
        }

        // Number: digits, optional dot and exponent
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && chars[i] == '.' {
                i += 1;
                while i < len && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Exponent
            if i < len && (chars[i] == 'E' || chars[i] == 'e') {
                i += 1;
                if i < len && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < len && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let num_str: String = chars[start..i].iter().collect();
            let n: f64 = num_str
                .parse()
                .map_err(|_| format!("Invalid number: {num_str}"))?;
            tokens.push(Token::Number(n));
            continue;
        }

        // Operators and punctuation
        match ch {
            '+' => {
                tokens.push(Token::Operator(Op::Add));
                i += 1;
                continue;
            }
            '-' => {
                tokens.push(Token::Operator(Op::Sub));
                i += 1;
                continue;
            }
            '*' => {
                tokens.push(Token::Operator(Op::Mul));
                i += 1;
                continue;
            }
            '/' => {
                tokens.push(Token::Operator(Op::Div));
                i += 1;
                continue;
            }
            '^' => {
                tokens.push(Token::Operator(Op::Pow));
                i += 1;
                continue;
            }
            '&' => {
                tokens.push(Token::Operator(Op::Concat));
                i += 1;
                continue;
            }
            '%' => {
                tokens.push(Token::Operator(Op::Percent));
                i += 1;
                continue;
            }
            '=' => {
                tokens.push(Token::Operator(Op::Eq));
                i += 1;
                continue;
            }
            '<' => {
                if i + 1 < len && chars[i + 1] == '>' {
                    tokens.push(Token::Operator(Op::Ne));
                    i += 2;
                } else if i + 1 < len && chars[i + 1] == '=' {
                    tokens.push(Token::Operator(Op::Le));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Op::Lt));
                    i += 1;
                }
                continue;
            }
            '>' => {
                if i + 1 < len && chars[i + 1] == '=' {
                    tokens.push(Token::Operator(Op::Ge));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Op::Gt));
                    i += 1;
                }
                continue;
            }
            '(' => {
                tokens.push(Token::OpenParen);
                i += 1;
                continue;
            }
            ')' => {
                tokens.push(Token::CloseParen);
                i += 1;
                continue;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
                continue;
            }
            _ => {}
        }

        // Identifiers: cell refs ($A$1), booleans (TRUE/FALSE), function names
        if ch == '$' || ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            // Scan the entire identifier-like region including $ for refs
            while i < len
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '$' || chars[i] == '_')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_ascii_uppercase();

            // Check for TRUE / FALSE
            if upper == "TRUE" {
                tokens.push(Token::Bool(true));
                continue;
            }
            if upper == "FALSE" {
                tokens.push(Token::Bool(false));
                continue;
            }

            // Check for sheet reference: word followed by !
            if i < len && chars[i] == '!' {
                let sheet_name = word;
                i += 1; // skip !
                        // Now parse the cell ref or range after the !
                let ref_start = i;
                while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '$') {
                    i += 1;
                }
                let ref_word: String = chars[ref_start..i].iter().collect();

                // Check for range (colon)
                if i < len && chars[i] == ':' {
                    i += 1;
                    let range_end_start = i;
                    while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '$') {
                        i += 1;
                    }
                    let end_word: String = chars[range_end_start..i].iter().collect();
                    let start_ref = parse_formula_ref(&ref_word)
                        .ok_or_else(|| format!("Invalid cell ref: {ref_word}"))?;
                    let end_ref = parse_formula_ref(&end_word)
                        .ok_or_else(|| format!("Invalid cell ref: {end_word}"))?;
                    tokens.push(Token::SheetRef(
                        sheet_name,
                        Box::new(Token::RangeRef(FormulaRange {
                            start: start_ref,
                            end: end_ref,
                        })),
                    ));
                } else {
                    let fref = parse_formula_ref(&ref_word)
                        .ok_or_else(|| format!("Invalid cell ref after sheet: {ref_word}"))?;
                    tokens.push(Token::SheetRef(sheet_name, Box::new(Token::CellRef(fref))));
                }
                continue;
            }

            // Check for function call: identifier followed by (
            if i < len && chars[i] == '(' {
                tokens.push(Token::Function(upper));
                // Don't consume the '(' — it will be handled as OpenParen next iteration
                continue;
            }

            // Try parsing as cell ref or range
            // Check if next char is ':' (range)
            if i < len && chars[i] == ':' {
                // Try as range start
                if let Some(start_ref) = parse_formula_ref(&word) {
                    i += 1; // skip ':'
                    let range_end_start = i;
                    while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '$') {
                        i += 1;
                    }
                    let end_word: String = chars[range_end_start..i].iter().collect();
                    let end_ref = parse_formula_ref(&end_word)
                        .ok_or_else(|| format!("Invalid range end: {end_word}"))?;
                    tokens.push(Token::RangeRef(FormulaRange {
                        start: start_ref,
                        end: end_ref,
                    }));
                    continue;
                }
            }

            // Try as plain cell ref
            if let Some(fref) = parse_formula_ref(&word) {
                tokens.push(Token::CellRef(fref));
                continue;
            }

            // Unknown identifier — treat as function name without parens (will error in parser)
            // or could be a named range (not implemented)
            return Err(format!("Unknown identifier: {word}"));
        }

        return Err(format!("Unexpected character: '{ch}'"));
    }

    Ok(tokens)
}

/// Parse a cell reference string that may contain `$` for absolute refs.
/// Handles: A1, $A1, A$1, $A$1
fn parse_formula_ref(s: &str) -> Option<FormulaRef> {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    // Optional $ before column
    let abs_col = if i < len && chars[i] == '$' {
        i += 1;
        true
    } else {
        false
    };

    // Column letters
    let mut col: u32 = 0;
    let mut col_len = 0;
    while i < len && chars[i].is_ascii_alphabetic() {
        col = col * 26 + (chars[i].to_ascii_uppercase() as u32 - b'A' as u32 + 1);
        i += 1;
        col_len += 1;
    }
    if col_len == 0 || col == 0 {
        return None;
    }

    // Optional $ before row
    let abs_row = if i < len && chars[i] == '$' {
        i += 1;
        true
    } else {
        false
    };

    // Row digits
    let row_start = i;
    while i < len && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == row_start || i != len {
        return None;
    }
    let row_str: String = chars[row_start..i].iter().collect();
    let row: u32 = row_str.parse().ok()?;
    if row == 0 {
        return None;
    }

    Some(FormulaRef {
        col: col - 1,
        row: row - 1,
        abs_col,
        abs_row,
    })
}

// ─── Parser ────────────────────────────────────────────────

/// Parse a formula string into an AST.
///
/// The leading `=` should already be stripped.
///
/// # Errors
///
/// Returns an error if the formula has syntax errors.
pub fn parse_formula(input: &str) -> Result<Expr, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err(format!(
            "Unexpected token after expression: {:?}",
            parser.tokens[parser.pos]
        ));
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        match self.advance() {
            Some(ref tok) if tok == expected => Ok(()),
            Some(tok) => Err(format!("Expected {expected:?}, got {tok:?}")),
            None => Err(format!("Expected {expected:?}, got end of input")),
        }
    }

    /// Parse expression (lowest precedence entry point).
    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_comparison()
    }

    /// Comparison: =, <>, <, >, <=, >=
    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_concat()?;
        while let Some(Token::Operator(
            op @ (Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge),
        )) = self.peek()
        {
            let op = *op;
            self.advance();
            let right = self.parse_concat()?;
            left = Expr::BinaryOp(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Concatenation: &
    fn parse_concat(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_add_sub()?;
        loop {
            if matches!(self.peek(), Some(Token::Operator(Op::Concat))) {
                self.advance();
                let right = self.parse_add_sub()?;
                left = Expr::BinaryOp(Op::Concat, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    /// Addition/subtraction: +, -
    fn parse_add_sub(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul_div()?;
        while let Some(Token::Operator(op @ (Op::Add | Op::Sub))) = self.peek() {
            let op = *op;
            self.advance();
            let right = self.parse_mul_div()?;
            left = Expr::BinaryOp(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Multiplication/division: *, /
    fn parse_mul_div(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;
        while let Some(Token::Operator(op @ (Op::Mul | Op::Div))) = self.peek() {
            let op = *op;
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Power: ^ (right-associative)
    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        if matches!(self.peek(), Some(Token::Operator(Op::Pow))) {
            self.advance();
            let exp = self.parse_power()?; // right-associative
            Ok(Expr::BinaryOp(Op::Pow, Box::new(base), Box::new(exp)))
        } else {
            Ok(base)
        }
    }

    /// Unary: +, -
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Operator(Op::Add)) => {
                self.advance();
                self.parse_percent()
            }
            Some(Token::Operator(Op::Sub)) => {
                self.advance();
                let operand = self.parse_percent()?;
                Ok(Expr::UnaryOp(Op::Sub, Box::new(operand)))
            }
            _ => self.parse_percent(),
        }
    }

    /// Percent postfix: expr%
    fn parse_percent(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        while matches!(self.peek(), Some(Token::Operator(Op::Percent))) {
            self.advance();
            expr = Expr::Percent(Box::new(expr));
        }
        Ok(expr)
    }

    /// Primary: literals, cell refs, ranges, function calls, parenthesized exprs.
    fn parse_primary(&mut self) -> Result<Expr, String> {
        let tok = self.advance().ok_or("Unexpected end of formula")?;
        match tok {
            Token::Number(n) => Ok(Expr::Literal(CellValue::Number(n))),
            Token::String(s) => Ok(Expr::Literal(CellValue::Text(s))),
            Token::Bool(b) => Ok(Expr::Literal(CellValue::Boolean(b))),
            Token::Error(e) => Ok(Expr::Literal(CellValue::Error(e))),
            Token::CellRef(fref) => Ok(Expr::CellRef(fref)),
            Token::RangeRef(frange) => Ok(Expr::Range(frange)),
            Token::SheetRef(sheet, inner) => match *inner {
                Token::CellRef(fref) => Ok(Expr::SheetRef(sheet, Box::new(Expr::CellRef(fref)))),
                Token::RangeRef(frange) => Ok(Expr::SheetRef(sheet, Box::new(Expr::Range(frange)))),
                _ => Err(format!(
                    "Expected cell or range after sheet ref, got {inner:?}"
                )),
            },
            Token::Function(name) => {
                self.expect(&Token::OpenParen)?;
                let mut args = Vec::new();
                if !matches!(self.peek(), Some(Token::CloseParen)) {
                    args.push(self.parse_expr()?);
                    while matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                        args.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::CloseParen)?;
                Ok(Expr::FunctionCall(name, args))
            }
            Token::OpenParen => {
                let expr = self.parse_expr()?;
                self.expect(&Token::CloseParen)?;
                Ok(expr)
            }
            other => Err(format!("Unexpected token: {other:?}")),
        }
    }
}

// ─── Cell Lookup trait ─────────────────────────────────────

/// Trait for resolving cell values during formula evaluation.
pub trait CellLookup {
    /// Get the value of a single cell.
    fn get_value(&self, cell: &CellRef) -> CellValue;

    /// Get all values in a rectangular range.
    fn get_range_values(&self, range: &CellRange) -> Vec<CellValue>;
}

// ─── Sheet Lookup trait (cross-sheet references) ────────────

/// Trait for resolving cross-sheet references during formula evaluation.
///
/// Implementations provide access to other sheets by name so that formulas like
/// `Sheet1!A1` or `Sheet2!A1:B5` can be resolved.
pub trait SheetLookup {
    /// Look up a sheet by name and return a `CellLookup` reference for it.
    /// Returns `None` if the sheet does not exist.
    fn get_sheet(&self, name: &str) -> Option<&dyn CellLookup>;
}

/// A combined context that provides both cell lookup (current sheet) and sheet lookup
/// (cross-sheet references).
pub struct EvalContext<'a> {
    /// The current sheet's cell lookup.
    pub current: &'a dyn CellLookup,
    /// Optional cross-sheet lookup.
    pub sheets: Option<&'a dyn SheetLookup>,
}

// ─── Formula Evaluator ─────────────────────────────────────

/// Stateless formula evaluation engine.
pub struct FormulaEngine;

impl FormulaEngine {
    /// Evaluate an expression tree using the given cell lookup context.
    pub fn evaluate(expr: &Expr, ctx: &dyn CellLookup) -> CellValue {
        FormulaEngine::evaluate_with_sheets(expr, ctx, None)
    }

    /// Evaluate an expression tree with optional cross-sheet lookup support.
    ///
    /// When `sheets` is `Some`, `SheetRef` nodes will resolve to the named sheet.
    /// When `sheets` is `None`, `SheetRef` nodes fall back to the current context.
    pub fn evaluate_with_sheets(
        expr: &Expr,
        ctx: &dyn CellLookup,
        sheets: Option<&dyn SheetLookup>,
    ) -> CellValue {
        match expr {
            Expr::Literal(v) => v.clone(),
            Expr::CellRef(fref) => ctx.get_value(&fref.to_cell_ref()),
            Expr::Range(_) => CellValue::Error(CellError::Value),
            Expr::SheetRef(sheet_name, inner) => {
                // Attempt to resolve the sheet via SheetLookup.
                if let Some(sheet_lookup) = sheets {
                    if let Some(target_ctx) = sheet_lookup.get_sheet(sheet_name) {
                        return FormulaEngine::evaluate_with_sheets(inner, target_ctx, sheets);
                    }
                    // Sheet not found
                    return CellValue::Error(CellError::Ref);
                }
                // No sheet lookup available — fall back to current context.
                FormulaEngine::evaluate_with_sheets(inner, ctx, sheets)
            }
            Expr::BinaryOp(op, left, right) => {
                eval_binary_with_sheets(*op, left, right, ctx, sheets)
            }
            Expr::UnaryOp(op, operand) => eval_unary_with_sheets(*op, operand, ctx, sheets),
            Expr::FunctionCall(name, args) => eval_function_with_sheets(name, args, ctx, sheets),
            Expr::Percent(inner) => {
                let v = FormulaEngine::evaluate_with_sheets(inner, ctx, sheets);
                match coerce_number(&v) {
                    Some(n) => CellValue::Number(n / 100.0),
                    None => CellValue::Error(CellError::Value),
                }
            }
        }
    }
}

/// Coerce a CellValue to f64 for arithmetic.
fn coerce_number(v: &CellValue) -> Option<f64> {
    match v {
        CellValue::Number(n) => Some(*n),
        CellValue::Date(n) => Some(*n),
        CellValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        CellValue::Empty => Some(0.0),
        CellValue::Text(s) => s.trim().parse::<f64>().ok(),
        CellValue::Error(_) => None,
    }
}

/// Coerce a CellValue to a string.
fn coerce_string(v: &CellValue) -> String {
    match v {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => {
            if *n == (*n as i64) as f64 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CellValue::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Empty => String::new(),
        CellValue::Error(e) => format!("{e}"),
        CellValue::Date(n) => format!("{n}"),
    }
}

/// Coerce a CellValue to bool for logical functions.
fn coerce_bool(v: &CellValue) -> Option<bool> {
    match v {
        CellValue::Boolean(b) => Some(*b),
        CellValue::Number(n) => Some(*n != 0.0),
        CellValue::Empty => Some(false),
        CellValue::Text(s) => match s.to_ascii_uppercase().as_str() {
            "TRUE" => Some(true),
            "FALSE" => Some(false),
            _ => None,
        },
        CellValue::Error(_) | CellValue::Date(_) => None,
    }
}

fn eval_binary_with_sheets(
    op: Op,
    left: &Expr,
    right: &Expr,
    ctx: &dyn CellLookup,
    sheets: Option<&dyn SheetLookup>,
) -> CellValue {
    let lv = FormulaEngine::evaluate_with_sheets(left, ctx, sheets);
    let rv = FormulaEngine::evaluate_with_sheets(right, ctx, sheets);

    // Propagate errors
    if let CellValue::Error(e) = &lv {
        return CellValue::Error(*e);
    }
    if let CellValue::Error(e) = &rv {
        return CellValue::Error(*e);
    }

    match op {
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Pow => {
            let ln = match coerce_number(&lv) {
                Some(n) => n,
                None => return CellValue::Error(CellError::Value),
            };
            let rn = match coerce_number(&rv) {
                Some(n) => n,
                None => return CellValue::Error(CellError::Value),
            };
            let result = match op {
                Op::Add => ln + rn,
                Op::Sub => ln - rn,
                Op::Mul => ln * rn,
                Op::Div => {
                    if rn == 0.0 {
                        return CellValue::Error(CellError::DivZero);
                    }
                    ln / rn
                }
                Op::Pow => ln.powf(rn),
                _ => unreachable!(),
            };
            CellValue::Number(result)
        }
        Op::Concat => {
            let ls = coerce_string(&lv);
            let rs = coerce_string(&rv);
            CellValue::Text(format!("{ls}{rs}"))
        }
        Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge => eval_comparison(op, &lv, &rv),
        Op::Percent => CellValue::Error(CellError::Value),
    }
}

fn eval_unary_with_sheets(
    op: Op,
    operand: &Expr,
    ctx: &dyn CellLookup,
    sheets: Option<&dyn SheetLookup>,
) -> CellValue {
    let v = FormulaEngine::evaluate_with_sheets(operand, ctx, sheets);
    if let CellValue::Error(e) = v {
        return CellValue::Error(e);
    }
    match op {
        Op::Sub => match coerce_number(&v) {
            Some(n) => CellValue::Number(-n),
            None => CellValue::Error(CellError::Value),
        },
        Op::Add => match coerce_number(&v) {
            Some(n) => CellValue::Number(n),
            None => CellValue::Error(CellError::Value),
        },
        _ => CellValue::Error(CellError::Value),
    }
}

fn eval_function_with_sheets(
    name: &str,
    args: &[Expr],
    ctx: &dyn CellLookup,
    sheets: Option<&dyn SheetLookup>,
) -> CellValue {
    // Resolve any SheetRef arguments by replacing them with the unwrapped
    // inner expression and switching the context for that argument.
    // For simple cases (single SheetRef arg in SUM, etc.) we resolve here.
    if let Some(sheet_lookup) = sheets {
        // Check if any arg is a SheetRef — if so, resolve it
        let mut resolved_args: Vec<Expr> = Vec::new();
        let mut resolved_ctx: Option<&dyn CellLookup> = None;
        let mut any_sheet_ref = false;

        for arg in args {
            if let Expr::SheetRef(sheet_name, inner) = arg {
                if let Some(target) = sheet_lookup.get_sheet(sheet_name) {
                    resolved_args.push((**inner).clone());
                    resolved_ctx = Some(target);
                    any_sheet_ref = true;
                } else {
                    return CellValue::Error(CellError::Ref);
                }
            } else {
                resolved_args.push(arg.clone());
            }
        }

        if any_sheet_ref {
            // Use the resolved context for function evaluation.
            // Note: this is simplified — if multiple args reference different sheets,
            // only the last resolved context is used. For most real formulas this works.
            let effective_ctx = resolved_ctx.unwrap_or(ctx);
            return eval_function(name, &resolved_args, effective_ctx);
        }
    }

    eval_function(name, args, ctx)
}

// NOTE: eval_binary replaced by eval_binary_with_sheets above.

fn eval_comparison(op: Op, lv: &CellValue, rv: &CellValue) -> CellValue {
    // Compare numbers if both are numeric
    if let (Some(ln), Some(rn)) = (coerce_number(lv), coerce_number(rv)) {
        // But only if at least one side is actually numeric (not two texts)
        let l_is_num = matches!(
            lv,
            CellValue::Number(_) | CellValue::Date(_) | CellValue::Boolean(_) | CellValue::Empty
        );
        let r_is_num = matches!(
            rv,
            CellValue::Number(_) | CellValue::Date(_) | CellValue::Boolean(_) | CellValue::Empty
        );
        if l_is_num && r_is_num {
            let result = match op {
                Op::Eq => (ln - rn).abs() < f64::EPSILON,
                Op::Ne => (ln - rn).abs() >= f64::EPSILON,
                Op::Lt => ln < rn,
                Op::Gt => ln > rn,
                Op::Le => ln <= rn,
                Op::Ge => ln >= rn,
                _ => false,
            };
            return CellValue::Boolean(result);
        }
    }

    // Fall back to string comparison (case-insensitive, as in Excel)
    let ls = coerce_string(lv).to_ascii_lowercase();
    let rs = coerce_string(rv).to_ascii_lowercase();
    let result = match op {
        Op::Eq => ls == rs,
        Op::Ne => ls != rs,
        Op::Lt => ls < rs,
        Op::Gt => ls > rs,
        Op::Le => ls <= rs,
        Op::Ge => ls >= rs,
        _ => false,
    };
    CellValue::Boolean(result)
}

// NOTE: eval_unary replaced by eval_unary_with_sheets above.

// ─── Function Evaluation ───────────────────────────────────

/// Collect numeric values from an argument that might be a range or a single value.
/// Skips Empty and Text cells in ranges (standard Excel behavior for aggregates).
fn collect_numbers(arg: &Expr, ctx: &dyn CellLookup) -> Vec<f64> {
    match arg {
        Expr::Range(frange) => {
            let range = frange.to_cell_range();
            let values = ctx.get_range_values(&range);
            values
                .iter()
                .filter(|v| !matches!(v, CellValue::Empty | CellValue::Text(_)))
                .filter_map(coerce_number)
                .collect()
        }
        Expr::SheetRef(_name, inner) => {
            // For cross-sheet range references, evaluate resolves the context.
            // Here we fall back to evaluating the inner expression in the current context.
            // The proper cross-sheet resolution happens in evaluate_with_sheets.
            collect_numbers(inner, ctx)
        }
        _ => {
            let v = FormulaEngine::evaluate(arg, ctx);
            match coerce_number(&v) {
                Some(n) => vec![n],
                None => vec![],
            }
        }
    }
}

/// Collect all CellValues from an argument (range-aware).
fn collect_values(arg: &Expr, ctx: &dyn CellLookup) -> Vec<CellValue> {
    match arg {
        Expr::Range(frange) => {
            let range = frange.to_cell_range();
            ctx.get_range_values(&range)
        }
        Expr::SheetRef(_name, inner) => {
            // For cross-sheet range references, fall back to inner evaluation.
            collect_values(inner, ctx)
        }
        _ => vec![FormulaEngine::evaluate(arg, ctx)],
    }
}

fn eval_function(name: &str, args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    match name {
        // ─── P0 Functions ───
        "SUM" => eval_sum(args, ctx),
        "AVERAGE" => eval_average(args, ctx),
        "MIN" => eval_min(args, ctx),
        "MAX" => eval_max(args, ctx),
        "COUNT" => eval_count(args, ctx),
        "COUNTA" => eval_counta(args, ctx),
        "IF" => eval_if(args, ctx),
        "AND" => eval_and(args, ctx),
        "OR" => eval_or(args, ctx),
        "NOT" => eval_not(args, ctx),
        "IFERROR" => eval_iferror(args, ctx),

        // ─── P1 Lookup Functions ───
        "VLOOKUP" => eval_vlookup(args, ctx),
        "HLOOKUP" => eval_hlookup(args, ctx),
        "INDEX" => eval_index(args, ctx),
        "MATCH" => eval_match(args, ctx),

        // ─── P1 String Functions ───
        "LEFT" => eval_left(args, ctx),
        "RIGHT" => eval_right(args, ctx),
        "MID" => eval_mid(args, ctx),
        "LEN" => eval_len(args, ctx),
        "TRIM" => eval_trim(args, ctx),
        "CONCATENATE" => eval_concatenate(args, ctx),
        "UPPER" => eval_upper(args, ctx),
        "LOWER" => eval_lower(args, ctx),

        // ─── P1 Math Functions ───
        "ROUND" => eval_round(args, ctx),
        "ABS" => eval_abs(args, ctx),
        "INT" => eval_int(args, ctx),
        "MOD" => eval_mod(args, ctx),
        "POWER" => eval_power(args, ctx),
        "SQRT" => eval_sqrt(args, ctx),

        // ─── P1 Conditional Aggregate Functions ───
        "COUNTIF" => eval_countif(args, ctx),
        "SUMIF" => eval_sumif(args, ctx),
        "AVERAGEIF" => eval_averageif(args, ctx),
        "COUNTIFS" => eval_countif(args, ctx), // alias for now
        "SUMIFS" => eval_sumif(args, ctx),

        // ─── P1 Date Functions ───
        "NOW" => eval_now(args),
        "TODAY" => eval_today(args),
        "DATE" => eval_date(args, ctx),
        "YEAR" => eval_year(args, ctx),
        "MONTH" => eval_month(args, ctx),
        "DAY" => eval_day(args, ctx),

        // ─── P2 String Functions ───
        "FIND" => eval_find(args, ctx, true),
        "SEARCH" => eval_find(args, ctx, false),
        "SUBSTITUTE" => eval_substitute(args, ctx),
        "TEXT" => eval_text(args, ctx),
        "VALUE" => eval_value(args, ctx),

        // ─── P2 Trig Functions ───
        "SIN" => eval_trig1(args, ctx, f64::sin),
        "COS" => eval_trig1(args, ctx, f64::cos),
        "TAN" => eval_trig1(args, ctx, f64::tan),
        "ASIN" => eval_trig1(args, ctx, f64::asin),
        "ACOS" => eval_trig1(args, ctx, f64::acos),
        "ATAN" => eval_trig1(args, ctx, f64::atan),

        // ─── P2 Log/Exp Functions ───
        "LOG" => eval_log(args, ctx),
        "LOG10" => eval_log10(args, ctx),
        "LN" => eval_ln(args, ctx),
        "EXP" => eval_exp(args, ctx),

        // ─── P2 Rounding Functions ───
        "CEILING" => eval_ceiling(args, ctx),
        "FLOOR" => eval_floor(args, ctx),

        _ => CellValue::Error(CellError::Name),
    }
}

// ─── P0 Function Implementations ───

fn eval_sum(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut total = 0.0;
    for arg in args {
        for n in collect_numbers(arg, ctx) {
            total += n;
        }
    }
    CellValue::Number(total)
}

fn eval_average(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut total = 0.0;
    let mut count = 0usize;
    for arg in args {
        for n in collect_numbers(arg, ctx) {
            total += n;
            count += 1;
        }
    }
    if count == 0 {
        CellValue::Error(CellError::DivZero)
    } else {
        CellValue::Number(total / count as f64)
    }
}

fn eval_min(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut min_val: Option<f64> = None;
    for arg in args {
        for n in collect_numbers(arg, ctx) {
            min_val = Some(match min_val {
                Some(cur) => cur.min(n),
                None => n,
            });
        }
    }
    match min_val {
        Some(n) => CellValue::Number(n),
        None => CellValue::Number(0.0),
    }
}

fn eval_max(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut max_val: Option<f64> = None;
    for arg in args {
        for n in collect_numbers(arg, ctx) {
            max_val = Some(match max_val {
                Some(cur) => cur.max(n),
                None => n,
            });
        }
    }
    match max_val {
        Some(n) => CellValue::Number(n),
        None => CellValue::Number(0.0),
    }
}

fn eval_count(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut count = 0usize;
    for arg in args {
        let values = collect_values(arg, ctx);
        for v in &values {
            if matches!(v, CellValue::Number(_) | CellValue::Date(_)) {
                count += 1;
            }
        }
    }
    CellValue::Number(count as f64)
}

fn eval_counta(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut count = 0usize;
    for arg in args {
        let values = collect_values(arg, ctx);
        for v in &values {
            if !matches!(v, CellValue::Empty) {
                count += 1;
            }
        }
    }
    CellValue::Number(count as f64)
}

fn eval_if(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.is_empty() || args.len() > 3 {
        return CellValue::Error(CellError::Value);
    }
    let cond = FormulaEngine::evaluate(&args[0], ctx);
    let cond_bool = match coerce_bool(&cond) {
        Some(b) => b,
        None => return CellValue::Error(CellError::Value),
    };
    if cond_bool {
        if args.len() > 1 {
            FormulaEngine::evaluate(&args[1], ctx)
        } else {
            CellValue::Boolean(true)
        }
    } else if args.len() > 2 {
        FormulaEngine::evaluate(&args[2], ctx)
    } else {
        CellValue::Boolean(false)
    }
}

fn eval_and(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.is_empty() {
        return CellValue::Error(CellError::Value);
    }
    for arg in args {
        let v = FormulaEngine::evaluate(arg, ctx);
        match coerce_bool(&v) {
            Some(false) => return CellValue::Boolean(false),
            None => return CellValue::Error(CellError::Value),
            _ => {}
        }
    }
    CellValue::Boolean(true)
}

fn eval_or(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.is_empty() {
        return CellValue::Error(CellError::Value);
    }
    for arg in args {
        let v = FormulaEngine::evaluate(arg, ctx);
        match coerce_bool(&v) {
            Some(true) => return CellValue::Boolean(true),
            None => return CellValue::Error(CellError::Value),
            _ => {}
        }
    }
    CellValue::Boolean(false)
}

fn eval_not(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    let v = FormulaEngine::evaluate(&args[0], ctx);
    match coerce_bool(&v) {
        Some(b) => CellValue::Boolean(!b),
        None => CellValue::Error(CellError::Value),
    }
}

fn eval_iferror(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let v = FormulaEngine::evaluate(&args[0], ctx);
    if matches!(v, CellValue::Error(_)) {
        FormulaEngine::evaluate(&args[1], ctx)
    } else {
        v
    }
}

// ─── P1 Lookup Function Implementations ───

fn eval_vlookup(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // VLOOKUP(lookup_value, table_range, col_index, [range_lookup])
    if args.len() < 3 || args.len() > 4 {
        return CellValue::Error(CellError::Value);
    }
    let lookup_val = FormulaEngine::evaluate(&args[0], ctx);
    let range = match &args[1] {
        Expr::Range(frange) => frange.to_cell_range(),
        _ => return CellValue::Error(CellError::Value),
    };
    let col_index = match FormulaEngine::evaluate(&args[2], ctx) {
        CellValue::Number(n) => n as usize,
        _ => return CellValue::Error(CellError::Value),
    };
    if col_index < 1 {
        return CellValue::Error(CellError::Value);
    }
    let exact = if args.len() == 4 {
        match coerce_bool(&FormulaEngine::evaluate(&args[3], ctx)) {
            Some(b) => !b, // FALSE means exact match
            None => false,
        }
    } else {
        false // default is approximate (range_lookup=TRUE)
    };

    let num_cols = (range.end.col as i64 - range.start.col as i64 + 1).max(0) as usize;
    if col_index > num_cols {
        return CellValue::Error(CellError::Ref);
    }

    // Search down the first column
    let mut last_match_row: Option<u32> = None;
    for row in range.start.row..=range.end.row {
        let cell_val = ctx.get_value(&CellRef::new(range.start.col, row));
        if values_equal(&lookup_val, &cell_val) {
            last_match_row = Some(row);
            if exact {
                break; // exact match found
            }
        } else if !exact {
            // For approximate match, keep going while values <= lookup_val
            if compare_values(&cell_val, &lookup_val) == Some(std::cmp::Ordering::Greater) {
                break;
            }
            // If cell_val <= lookup_val, this could be a candidate
            if compare_values(&cell_val, &lookup_val)
                .is_some_and(|ord| ord != std::cmp::Ordering::Greater)
            {
                last_match_row = Some(row);
            }
        }
    }

    match last_match_row {
        Some(row) => {
            let result_col = range.start.col + (col_index as u32 - 1);
            ctx.get_value(&CellRef::new(result_col, row))
        }
        None => CellValue::Error(CellError::NA),
    }
}

fn eval_hlookup(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // HLOOKUP(lookup_value, table_range, row_index, [range_lookup])
    if args.len() < 3 || args.len() > 4 {
        return CellValue::Error(CellError::Value);
    }
    let lookup_val = FormulaEngine::evaluate(&args[0], ctx);
    let range = match &args[1] {
        Expr::Range(frange) => frange.to_cell_range(),
        _ => return CellValue::Error(CellError::Value),
    };
    let row_index = match FormulaEngine::evaluate(&args[2], ctx) {
        CellValue::Number(n) => n as usize,
        _ => return CellValue::Error(CellError::Value),
    };
    if row_index < 1 {
        return CellValue::Error(CellError::Value);
    }
    let exact = if args.len() == 4 {
        match coerce_bool(&FormulaEngine::evaluate(&args[3], ctx)) {
            Some(b) => !b,
            None => false,
        }
    } else {
        false
    };

    let num_rows = (range.end.row as i64 - range.start.row as i64 + 1).max(0) as usize;
    if row_index > num_rows {
        return CellValue::Error(CellError::Ref);
    }

    // Search across the first row
    let mut last_match_col: Option<u32> = None;
    for col in range.start.col..=range.end.col {
        let cell_val = ctx.get_value(&CellRef::new(col, range.start.row));
        if values_equal(&lookup_val, &cell_val) {
            last_match_col = Some(col);
            if exact {
                break;
            }
        } else if !exact {
            if compare_values(&cell_val, &lookup_val) == Some(std::cmp::Ordering::Greater) {
                break;
            }
            if compare_values(&cell_val, &lookup_val)
                .is_some_and(|ord| ord != std::cmp::Ordering::Greater)
            {
                last_match_col = Some(col);
            }
        }
    }

    match last_match_col {
        Some(col) => {
            let result_row = range.start.row + (row_index as u32 - 1);
            ctx.get_value(&CellRef::new(col, result_row))
        }
        None => CellValue::Error(CellError::NA),
    }
}

fn eval_index(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // INDEX(range, row_num, [col_num])
    if args.is_empty() || args.len() > 3 {
        return CellValue::Error(CellError::Value);
    }
    let range = match &args[0] {
        Expr::Range(frange) => frange.to_cell_range(),
        _ => return CellValue::Error(CellError::Value),
    };
    let row_num = if args.len() > 1 {
        match FormulaEngine::evaluate(&args[1], ctx) {
            CellValue::Number(n) => n as u32,
            _ => return CellValue::Error(CellError::Value),
        }
    } else {
        1
    };
    let col_num = if args.len() > 2 {
        match FormulaEngine::evaluate(&args[2], ctx) {
            CellValue::Number(n) => n as u32,
            _ => return CellValue::Error(CellError::Value),
        }
    } else {
        1
    };
    if row_num < 1 || col_num < 1 {
        return CellValue::Error(CellError::Value);
    }
    let target_row = range.start.row + row_num - 1;
    let target_col = range.start.col + col_num - 1;
    if target_row > range.end.row || target_col > range.end.col {
        return CellValue::Error(CellError::Ref);
    }
    ctx.get_value(&CellRef::new(target_col, target_row))
}

fn eval_match(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // MATCH(lookup_value, lookup_range, [match_type])
    if args.is_empty() || args.len() > 3 {
        return CellValue::Error(CellError::Value);
    }
    let lookup_val = FormulaEngine::evaluate(&args[0], ctx);
    let range = match &args[1] {
        Expr::Range(frange) => frange.to_cell_range(),
        _ => return CellValue::Error(CellError::Value),
    };
    let match_type = if args.len() > 2 {
        match FormulaEngine::evaluate(&args[2], ctx) {
            CellValue::Number(n) => n as i32,
            _ => 1,
        }
    } else {
        1 // default
    };

    // Determine if it's a row vector or column vector
    let is_row = range.start.row == range.end.row;
    let count = if is_row {
        (range.end.col - range.start.col + 1) as usize
    } else {
        (range.end.row - range.start.row + 1) as usize
    };

    let get_nth = |i: usize| -> CellValue {
        if is_row {
            ctx.get_value(&CellRef::new(range.start.col + i as u32, range.start.row))
        } else {
            ctx.get_value(&CellRef::new(range.start.col, range.start.row + i as u32))
        }
    };

    match match_type {
        0 => {
            // Exact match
            for i in 0..count {
                if values_equal(&lookup_val, &get_nth(i)) {
                    return CellValue::Number((i + 1) as f64);
                }
            }
            CellValue::Error(CellError::NA)
        }
        1 => {
            // Largest value <= lookup_val (data must be sorted ascending)
            let mut last_match: Option<usize> = None;
            for i in 0..count {
                let v = get_nth(i);
                if compare_values(&v, &lookup_val)
                    .is_some_and(|ord| ord != std::cmp::Ordering::Greater)
                {
                    last_match = Some(i);
                } else {
                    break;
                }
            }
            match last_match {
                Some(i) => CellValue::Number((i + 1) as f64),
                None => CellValue::Error(CellError::NA),
            }
        }
        -1 => {
            // Smallest value >= lookup_val (data must be sorted descending)
            let mut last_match: Option<usize> = None;
            for i in 0..count {
                let v = get_nth(i);
                if compare_values(&v, &lookup_val)
                    .is_some_and(|ord| ord != std::cmp::Ordering::Less)
                {
                    last_match = Some(i);
                } else {
                    break;
                }
            }
            match last_match {
                Some(i) => CellValue::Number((i + 1) as f64),
                None => CellValue::Error(CellError::NA),
            }
        }
        _ => CellValue::Error(CellError::Value),
    }
}

// ─── P1 String Function Implementations ───

fn eval_left(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.is_empty() || args.len() > 2 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    let num = if args.len() > 1 {
        match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
            Some(n) if n >= 0.0 => n as usize,
            _ => return CellValue::Error(CellError::Value),
        }
    } else {
        1
    };
    let chars: Vec<char> = s.chars().collect();
    let end = num.min(chars.len());
    CellValue::Text(chars[..end].iter().collect())
}

fn eval_right(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.is_empty() || args.len() > 2 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    let num = if args.len() > 1 {
        match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
            Some(n) if n >= 0.0 => n as usize,
            _ => return CellValue::Error(CellError::Value),
        }
    } else {
        1
    };
    let chars: Vec<char> = s.chars().collect();
    let start = chars.len().saturating_sub(num);
    CellValue::Text(chars[start..].iter().collect())
}

fn eval_mid(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 3 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    let start = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(n) if n >= 1.0 => (n as usize) - 1,
        _ => return CellValue::Error(CellError::Value),
    };
    let num = match coerce_number(&FormulaEngine::evaluate(&args[2], ctx)) {
        Some(n) if n >= 0.0 => n as usize,
        _ => return CellValue::Error(CellError::Value),
    };
    let chars: Vec<char> = s.chars().collect();
    let actual_start = start.min(chars.len());
    let actual_end = (actual_start + num).min(chars.len());
    CellValue::Text(chars[actual_start..actual_end].iter().collect())
}

fn eval_len(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    CellValue::Number(s.chars().count() as f64)
}

fn eval_trim(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    // Excel TRIM removes leading/trailing spaces and collapses internal multiple spaces to one
    let trimmed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    CellValue::Text(trimmed)
}

fn eval_concatenate(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    let mut result = String::new();
    for arg in args {
        let v = FormulaEngine::evaluate(arg, ctx);
        if let CellValue::Error(e) = v {
            return CellValue::Error(e);
        }
        result.push_str(&coerce_string(&v));
    }
    CellValue::Text(result)
}

fn eval_upper(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    CellValue::Text(s.to_uppercase())
}

fn eval_lower(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    let s = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    CellValue::Text(s.to_lowercase())
}

// ─── P1 Math Function Implementations ───

fn eval_round(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let n = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => n,
        None => return CellValue::Error(CellError::Value),
    };
    let digits = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(d) => d as i32,
        None => return CellValue::Error(CellError::Value),
    };
    let factor = 10f64.powi(digits);
    CellValue::Number((n * factor).round() / factor)
}

fn eval_abs(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => CellValue::Number(n.abs()),
        None => CellValue::Error(CellError::Value),
    }
}

fn eval_int(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => CellValue::Number(n.floor()),
        None => CellValue::Error(CellError::Value),
    }
}

fn eval_mod(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let n = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => n,
        None => return CellValue::Error(CellError::Value),
    };
    let d = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(d) => d,
        None => return CellValue::Error(CellError::Value),
    };
    if d == 0.0 {
        return CellValue::Error(CellError::DivZero);
    }
    // Excel MOD: result has the sign of the divisor
    let result = n - d * (n / d).floor();
    CellValue::Number(result)
}

fn eval_power(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let base = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => n,
        None => return CellValue::Error(CellError::Value),
    };
    let exp = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(n) => n,
        None => return CellValue::Error(CellError::Value),
    };
    let result = base.powf(exp);
    if result.is_nan() || result.is_infinite() {
        CellValue::Error(CellError::Num)
    } else {
        CellValue::Number(result)
    }
}

fn eval_sqrt(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) if n >= 0.0 => CellValue::Number(n.sqrt()),
        Some(_) => CellValue::Error(CellError::Num),
        None => CellValue::Error(CellError::Value),
    }
}

// ─── P1 Date Function Implementations ───

/// Excel serial date epoch: 1899-12-30 (day 0).
/// Day 1 = 1900-01-01. Excel erroneously treats 1900 as a leap year (day 60 = Feb 29, 1900).
///
/// Count days from 0000-03-01 to the given date using a well-known algorithm.
/// This avoids JDN complexities and matches Excel's date serial system.
fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    // Adjust month/year for overflow
    let mut y = year as i64;
    let mut m = month as i64;
    if m < 1 {
        let adj = (1 - m) / 12 + 1;
        y -= adj;
        m += adj * 12;
    }
    if m > 12 {
        y += (m - 1) / 12;
        m = (m - 1) % 12 + 1;
    }

    // Shift March=0 system
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    // Days in prior years + days in prior months + day
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64; // year of era [0, 399]
    let doy = (153 * m as u64 + 2) / 5 + day as u64 - 1; // day of year [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // day of era [0, 146096]
    era * 146097 + doe as i64 - 719468 // shift to 1970-01-01 epoch
}

/// Convert (year, month, day) to Excel serial date number.
fn date_to_serial(year: i32, month: i32, day: i32) -> f64 {
    // Days since 1970-01-01 (Unix epoch)
    let unix_days = days_from_civil(year, month, day);
    // In a correct (no-bug) system: 1900-01-01 = day 1, 1970-01-01 = day 25568.
    // Excel adds a phantom Feb 29, 1900 (serial 60), so dates from 1900-03-01 onward
    // are all +1 from the correct serial.
    // Correct serial (no phantom day):
    let serial = unix_days + 25568;
    // Add 1 for the phantom leap day for dates after 1900-02-28 (serial > 59)
    if serial > 59 {
        (serial + 1) as f64
    } else {
        serial as f64
    }
}

/// Convert Excel serial date number to (year, month, day).
fn serial_to_date(serial: f64) -> (i32, i32, i32) {
    let s = serial as i64;
    // Handle Excel's phantom Feb 29, 1900
    if s == 60 {
        return (1900, 2, 29); // Non-existent date, but Excel expects it
    }
    // For serial > 60, subtract 1 to undo the phantom leap day
    let adjusted = if s > 60 { s - 1 } else { s };
    // Convert serial to Unix days: correct serial 1 = 1900-01-01, offset = 25568
    let unix_days = adjusted - 25568;
    // Convert Unix days to civil date
    civil_from_days(unix_days)
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

fn eval_now(_args: &[Expr]) -> CellValue {
    // Use system clock to compute the current date/time as an Excel serial number.
    use std::time::{SystemTime, UNIX_EPOCH};
    if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let days_since_unix = duration.as_secs_f64() / 86400.0;
        // Unix epoch (1970-01-01) = Excel serial 25569 (accounts for phantom leap day)
        let serial = days_since_unix + 25569.0;
        CellValue::Number(serial)
    } else {
        CellValue::Error(CellError::Value)
    }
}

fn eval_today(_args: &[Expr]) -> CellValue {
    use std::time::{SystemTime, UNIX_EPOCH};
    if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let days_since_unix = (duration.as_secs() / 86400) as f64;
        // Unix epoch (1970-01-01) = Excel serial 25569 (accounts for phantom leap day)
        let serial = days_since_unix + 25569.0;
        CellValue::Number(serial)
    } else {
        CellValue::Error(CellError::Value)
    }
}

fn eval_date(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 3 {
        return CellValue::Error(CellError::Value);
    }
    let year = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => n as i32,
        None => return CellValue::Error(CellError::Value),
    };
    let month = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(n) => n as i32,
        None => return CellValue::Error(CellError::Value),
    };
    let day = match coerce_number(&FormulaEngine::evaluate(&args[2], ctx)) {
        Some(n) => n as i32,
        None => return CellValue::Error(CellError::Value),
    };
    // Excel: years 0-29 mean 2000-2029, 30-99 mean 1930-1999
    let adjusted_year = if (0..=29).contains(&year) {
        year + 2000
    } else if (30..=99).contains(&year) {
        year + 1900
    } else {
        year
    };
    CellValue::Number(date_to_serial(adjusted_year, month, day))
}

fn eval_year(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(serial) => {
            let (y, _, _) = serial_to_date(serial);
            CellValue::Number(y as f64)
        }
        None => CellValue::Error(CellError::Value),
    }
}

fn eval_month(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(serial) => {
            let (_, m, _) = serial_to_date(serial);
            CellValue::Number(m as f64)
        }
        None => CellValue::Error(CellError::Value),
    }
}

fn eval_day(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(serial) => {
            let (_, _, d) = serial_to_date(serial);
            CellValue::Number(d as f64)
        }
        None => CellValue::Error(CellError::Value),
    }
}

// ─── P1 Conditional Aggregate Function Implementations ───

/// Parse a criteria string into a comparison operator and a target value.
/// Supports: ">5", ">=10", "<3", "<=7", "<>0", "=5", plain number, "text*" wildcard.
fn parse_criteria(criteria: &CellValue) -> CriteriaMatch {
    let s = coerce_string(criteria);
    let trimmed = s.trim();

    // Operator-prefixed criteria: ">=10", ">5", "<=3", "<>0", "<5", "=5"
    if let Some(rest) = trimmed.strip_prefix(">=") {
        if let Ok(n) = rest.trim().parse::<f64>() {
            return CriteriaMatch::NumericOp(Op::Ge, n);
        }
    } else if let Some(rest) = trimmed.strip_prefix("<=") {
        if let Ok(n) = rest.trim().parse::<f64>() {
            return CriteriaMatch::NumericOp(Op::Le, n);
        }
    } else if let Some(rest) = trimmed.strip_prefix("<>") {
        let rhs = rest.trim();
        if let Ok(n) = rhs.parse::<f64>() {
            return CriteriaMatch::NumericOp(Op::Ne, n);
        }
        return CriteriaMatch::TextNotEqual(rhs.to_ascii_lowercase());
    } else if let Some(rest) = trimmed.strip_prefix('>') {
        if let Ok(n) = rest.trim().parse::<f64>() {
            return CriteriaMatch::NumericOp(Op::Gt, n);
        }
    } else if let Some(rest) = trimmed.strip_prefix('<') {
        if let Ok(n) = rest.trim().parse::<f64>() {
            return CriteriaMatch::NumericOp(Op::Lt, n);
        }
    } else if let Some(rest) = trimmed.strip_prefix('=') {
        let rhs = rest.trim();
        if let Ok(n) = rhs.parse::<f64>() {
            return CriteriaMatch::NumericOp(Op::Eq, n);
        }
        return CriteriaMatch::ExactText(rhs.to_ascii_lowercase());
    }

    // Wildcard match: contains * or ?
    if trimmed.contains('*') || trimmed.contains('?') {
        return CriteriaMatch::Wildcard(trimmed.to_ascii_lowercase());
    }

    // Plain number
    if let Ok(n) = trimmed.parse::<f64>() {
        return CriteriaMatch::NumericOp(Op::Eq, n);
    }

    // Numeric criteria passed as a Number CellValue
    if let CellValue::Number(n) = criteria {
        return CriteriaMatch::NumericOp(Op::Eq, *n);
    }

    // Plain text (exact match, case-insensitive)
    CriteriaMatch::ExactText(trimmed.to_ascii_lowercase())
}

/// Criteria matching modes for COUNTIF/SUMIF/AVERAGEIF.
enum CriteriaMatch {
    NumericOp(Op, f64),
    ExactText(String),
    TextNotEqual(String),
    Wildcard(String),
}

impl CriteriaMatch {
    /// Check whether a cell value matches this criteria.
    fn matches(&self, val: &CellValue) -> bool {
        match self {
            CriteriaMatch::NumericOp(op, target) => {
                if let Some(n) = coerce_number(val) {
                    match op {
                        Op::Eq => (n - target).abs() < f64::EPSILON,
                        Op::Ne => (n - target).abs() >= f64::EPSILON,
                        Op::Lt => n < *target,
                        Op::Gt => n > *target,
                        Op::Le => n <= *target,
                        Op::Ge => n >= *target,
                        _ => false,
                    }
                } else {
                    false
                }
            }
            CriteriaMatch::ExactText(target) => {
                let s = coerce_string(val).to_ascii_lowercase();
                s == *target
            }
            CriteriaMatch::TextNotEqual(target) => {
                let s = coerce_string(val).to_ascii_lowercase();
                s != *target
            }
            CriteriaMatch::Wildcard(pattern) => {
                let s = coerce_string(val).to_ascii_lowercase();
                wildcard_match(&s, pattern)
            }
        }
    }
}

/// Simple wildcard matching: `*` matches any sequence, `?` matches any single character.
fn wildcard_match(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pattern.chars().collect();
    let (tlen, plen) = (t.len(), p.len());
    let mut ti = 0;
    let mut pi = 0;
    let mut star_pi: Option<usize> = None;
    let mut star_ti: usize = 0;

    while ti < tlen {
        if pi < plen && (p[pi] == '?' || p[pi] == t[ti]) {
            ti += 1;
            pi += 1;
        } else if pi < plen && p[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < plen && p[pi] == '*' {
        pi += 1;
    }
    pi == plen
}

fn eval_countif(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // COUNTIF(range, criteria)
    if args.len() < 2 {
        return CellValue::Error(CellError::Value);
    }
    let values = collect_values(&args[0], ctx);
    let criteria_val = FormulaEngine::evaluate(&args[1], ctx);
    let criteria = parse_criteria(&criteria_val);

    let count = values.iter().filter(|v| criteria.matches(v)).count();
    CellValue::Number(count as f64)
}

fn eval_sumif(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // SUMIF(criteria_range, criteria, [sum_range])
    if args.len() < 2 || args.len() > 3 {
        return CellValue::Error(CellError::Value);
    }
    let criteria_values = collect_values(&args[0], ctx);
    let criteria_val = FormulaEngine::evaluate(&args[1], ctx);
    let criteria = parse_criteria(&criteria_val);

    let sum_values = if args.len() == 3 {
        collect_values(&args[2], ctx)
    } else {
        criteria_values.clone()
    };

    let mut total = 0.0;
    for (i, cv) in criteria_values.iter().enumerate() {
        if criteria.matches(cv) {
            if let Some(sv) = sum_values.get(i) {
                if let Some(n) = coerce_number(sv) {
                    total += n;
                }
            }
        }
    }
    CellValue::Number(total)
}

fn eval_averageif(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    // AVERAGEIF(criteria_range, criteria, [average_range])
    if args.len() < 2 || args.len() > 3 {
        return CellValue::Error(CellError::Value);
    }
    let criteria_values = collect_values(&args[0], ctx);
    let criteria_val = FormulaEngine::evaluate(&args[1], ctx);
    let criteria = parse_criteria(&criteria_val);

    let avg_values = if args.len() == 3 {
        collect_values(&args[2], ctx)
    } else {
        criteria_values.clone()
    };

    let mut total = 0.0;
    let mut count = 0usize;
    for (i, cv) in criteria_values.iter().enumerate() {
        if criteria.matches(cv) {
            if let Some(sv) = avg_values.get(i) {
                if let Some(n) = coerce_number(sv) {
                    total += n;
                    count += 1;
                }
            }
        }
    }
    if count == 0 {
        CellValue::Error(CellError::DivZero)
    } else {
        CellValue::Number(total / count as f64)
    }
}

// ─── P2 Function Implementations ──────────────────────────

/// FIND(find_text, within_text, [start_num]) — case-sensitive
/// SEARCH(find_text, within_text, [start_num]) — case-insensitive
fn eval_find(args: &[Expr], ctx: &dyn CellLookup, case_sensitive: bool) -> CellValue {
    if args.is_empty() || args.len() > 3 {
        return CellValue::Error(CellError::Value);
    }
    let find_text = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    let within_text = coerce_string(&FormulaEngine::evaluate(&args[1], ctx));
    let start_num = if args.len() > 2 {
        match coerce_number(&FormulaEngine::evaluate(&args[2], ctx)) {
            Some(n) if n >= 1.0 => (n as usize) - 1,
            _ => return CellValue::Error(CellError::Value),
        }
    } else {
        0
    };

    if start_num > within_text.len() {
        return CellValue::Error(CellError::Value);
    }

    let haystack = &within_text[start_num..];
    let pos = if case_sensitive {
        haystack.find(&find_text)
    } else {
        haystack
            .to_ascii_lowercase()
            .find(&find_text.to_ascii_lowercase())
    };

    match pos {
        Some(p) => CellValue::Number((start_num + p + 1) as f64),
        None => CellValue::Error(CellError::Value),
    }
}

/// SUBSTITUTE(text, old_text, new_text, [instance_num])
fn eval_substitute(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() < 3 || args.len() > 4 {
        return CellValue::Error(CellError::Value);
    }
    let text = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    let old_text = coerce_string(&FormulaEngine::evaluate(&args[1], ctx));
    let new_text = coerce_string(&FormulaEngine::evaluate(&args[2], ctx));

    if old_text.is_empty() {
        return CellValue::Text(text);
    }

    if args.len() == 4 {
        // Replace only the Nth instance
        let instance_num = match coerce_number(&FormulaEngine::evaluate(&args[3], ctx)) {
            Some(n) if n >= 1.0 => n as usize,
            _ => return CellValue::Error(CellError::Value),
        };
        let mut result = String::new();
        let mut count = 0usize;
        let mut remaining = text.as_str();
        while let Some(pos) = remaining.find(&old_text) {
            count += 1;
            if count == instance_num {
                result.push_str(&remaining[..pos]);
                result.push_str(&new_text);
                result.push_str(&remaining[pos + old_text.len()..]);
                return CellValue::Text(result);
            }
            result.push_str(&remaining[..pos + old_text.len()]);
            remaining = &remaining[pos + old_text.len()..];
        }
        result.push_str(remaining);
        CellValue::Text(result)
    } else {
        // Replace all instances
        CellValue::Text(text.replace(&old_text, &new_text))
    }
}

/// Single-argument trig function dispatcher.
fn eval_trig1(args: &[Expr], ctx: &dyn CellLookup, f: fn(f64) -> f64) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => {
            let result = f(n);
            if result.is_finite() {
                CellValue::Number(result)
            } else {
                CellValue::Error(CellError::Num)
            }
        }
        None => CellValue::Error(CellError::Value),
    }
}

/// LOG(number, [base]) — default base 10.
fn eval_log(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.is_empty() || args.len() > 2 {
        return CellValue::Error(CellError::Value);
    }
    let number = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) if n > 0.0 => n,
        _ => return CellValue::Error(CellError::Num),
    };
    let base = if args.len() > 1 {
        match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
            Some(b) if b > 0.0 && (b - 1.0).abs() > f64::EPSILON => b,
            _ => return CellValue::Error(CellError::Num),
        }
    } else {
        10.0
    };
    CellValue::Number(number.log(base))
}

/// LOG10(number)
fn eval_log10(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) if n > 0.0 => CellValue::Number(n.log10()),
        Some(_) => CellValue::Error(CellError::Num),
        None => CellValue::Error(CellError::Value),
    }
}

/// LN(number)
fn eval_ln(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) if n > 0.0 => CellValue::Number(n.ln()),
        Some(_) => CellValue::Error(CellError::Num),
        None => CellValue::Error(CellError::Value),
    }
}

/// EXP(number)
fn eval_exp(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => {
            let result = n.exp();
            if result.is_finite() {
                CellValue::Number(result)
            } else {
                CellValue::Error(CellError::Num)
            }
        }
        None => CellValue::Error(CellError::Value),
    }
}

/// CEILING(number, significance)
fn eval_ceiling(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let number = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => n,
        None => return CellValue::Error(CellError::Value),
    };
    let significance = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(s) if s.abs() > f64::EPSILON => s,
        _ => return CellValue::Error(CellError::Num),
    };
    // If signs differ, it's an error in Excel
    if number > 0.0 && significance < 0.0 {
        return CellValue::Error(CellError::Num);
    }
    CellValue::Number((number / significance).ceil() * significance)
}

/// FLOOR(number, significance)
fn eval_floor(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let number = match coerce_number(&FormulaEngine::evaluate(&args[0], ctx)) {
        Some(n) => n,
        None => return CellValue::Error(CellError::Value),
    };
    let significance = match coerce_number(&FormulaEngine::evaluate(&args[1], ctx)) {
        Some(s) if s.abs() > f64::EPSILON => s,
        _ => return CellValue::Error(CellError::Num),
    };
    if number > 0.0 && significance < 0.0 {
        return CellValue::Error(CellError::Num);
    }
    CellValue::Number((number / significance).floor() * significance)
}

/// TEXT(value, format_text) — basic number formatting.
fn eval_text(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 2 {
        return CellValue::Error(CellError::Value);
    }
    let value = FormulaEngine::evaluate(&args[0], ctx);
    let format = coerce_string(&FormulaEngine::evaluate(&args[1], ctx));
    let num = match coerce_number(&value) {
        Some(n) => n,
        None => return CellValue::Text(coerce_string(&value)),
    };
    let fmt_lower = format.to_ascii_lowercase();
    // Basic format patterns
    let result = if fmt_lower == "0" {
        format!("{:.0}", num)
    } else if fmt_lower == "0.0" {
        format!("{:.1}", num)
    } else if fmt_lower == "0.00" {
        format!("{:.2}", num)
    } else if fmt_lower == "0.000" {
        format!("{:.3}", num)
    } else if fmt_lower == "#,##0" {
        format_number_with_commas(num, 0)
    } else if fmt_lower == "#,##0.00" {
        format_number_with_commas(num, 2)
    } else if fmt_lower == "0%" || fmt_lower == "0.0%" || fmt_lower == "0.00%" {
        let decimals = fmt_lower.matches('.').count();
        let pct = num * 100.0;
        let decimal_places = if decimals > 0 {
            fmt_lower.len() - fmt_lower.find('.').unwrap() - 2
        } else {
            0
        };
        format!("{:.prec$}%", pct, prec = decimal_places)
    } else if fmt_lower == "0.00e+00" {
        format!("{:.2E}", num)
    } else {
        // Fallback: just convert to string
        format!("{}", num)
    };
    CellValue::Text(result)
}

/// Helper for TEXT() — format with thousands separator.
fn format_number_with_commas(num: f64, decimals: usize) -> String {
    let formatted = format!("{:.prec$}", num.abs(), prec = decimals);
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let mut with_commas = String::new();
    for (i, ch) in integer_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(ch);
    }
    let result: String = with_commas.chars().rev().collect();
    let sign = if num < 0.0 { "-" } else { "" };
    if parts.len() > 1 {
        format!("{}{}.{}", sign, result, parts[1])
    } else {
        format!("{}{}", sign, result)
    }
}

/// VALUE(text) — convert text to number.
fn eval_value(args: &[Expr], ctx: &dyn CellLookup) -> CellValue {
    if args.len() != 1 {
        return CellValue::Error(CellError::Value);
    }
    let text = coerce_string(&FormulaEngine::evaluate(&args[0], ctx));
    let trimmed = text.trim();
    // Handle percentage
    if let Some(stripped) = trimmed.strip_suffix('%') {
        if let Ok(n) = stripped.trim().parse::<f64>() {
            return CellValue::Number(n / 100.0);
        }
    }
    // Handle currency prefix ($)
    let cleaned = trimmed.trim_start_matches('$').replace(',', "");
    match cleaned.trim().parse::<f64>() {
        Ok(n) => CellValue::Number(n),
        Err(_) => CellValue::Error(CellError::Value),
    }
}

// ─── Helper Functions ──────────────────────────────────────

/// Test equality of two cell values (case-insensitive for strings).
fn values_equal(a: &CellValue, b: &CellValue) -> bool {
    match (a, b) {
        (CellValue::Number(x), CellValue::Number(y)) => (x - y).abs() < f64::EPSILON,
        (CellValue::Text(x), CellValue::Text(y)) => x.eq_ignore_ascii_case(y),
        (CellValue::Boolean(x), CellValue::Boolean(y)) => x == y,
        (CellValue::Empty, CellValue::Empty) => true,
        // Cross-type: try numeric
        _ => {
            if let (Some(x), Some(y)) = (coerce_number(a), coerce_number(b)) {
                (x - y).abs() < f64::EPSILON
            } else {
                false
            }
        }
    }
}

/// Compare two cell values for ordering.
fn compare_values(a: &CellValue, b: &CellValue) -> Option<std::cmp::Ordering> {
    // Try numeric comparison first
    if let (Some(x), Some(y)) = (coerce_number(a), coerce_number(b)) {
        return x.partial_cmp(&y);
    }
    // Fall back to string comparison
    let sa = coerce_string(a).to_ascii_lowercase();
    let sb = coerce_string(b).to_ascii_lowercase();
    Some(sa.cmp(&sb))
}

// ─── Dependency Graph ──────────────────────────────────────

/// Tracks which cells depend on which other cells for formula evaluation order.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// cell -> cells it depends on (forward edges).
    deps: HashMap<CellRef, Vec<CellRef>>,
    /// cell -> cells that depend on it (reverse edges).
    rdeps: HashMap<CellRef, Vec<CellRef>>,
}

impl DependencyGraph {
    /// Build a dependency graph from all formulas in a sheet.
    pub fn build(sheet: &crate::model::Sheet) -> Self {
        let mut graph = Self::default();

        for (cell_ref, cell) in &sheet.cells {
            if let Some(ref formula) = cell.formula {
                let refs = extract_references(formula);
                graph.deps.insert(*cell_ref, refs.clone());
                for dep in &refs {
                    graph.rdeps.entry(*dep).or_default().push(*cell_ref);
                }
            }
        }

        graph
    }

    /// Return a topological ordering of cells for evaluation.
    ///
    /// Returns `Ok(order)` on success, or `Err(cell)` if a circular reference is detected,
    /// where `cell` is the first cell found in the cycle.
    pub fn topological_order(&self) -> Result<Vec<CellRef>, CellRef> {
        // Kahn's algorithm
        let mut in_degree: HashMap<CellRef, usize> = HashMap::new();

        // Initialize in-degrees for all cells that have formulas
        for cell in self.deps.keys() {
            in_degree.entry(*cell).or_insert(0);
        }
        // Count incoming edges
        for (cell, deps) in &self.deps {
            // Only count deps that are also formula cells (in our graph)
            let dep_count = deps.iter().filter(|d| self.deps.contains_key(d)).count();
            in_degree.insert(*cell, dep_count);
        }

        let mut queue: VecDeque<CellRef> = VecDeque::new();
        for (cell, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(*cell);
            }
        }

        let mut order = Vec::new();
        while let Some(cell) = queue.pop_front() {
            order.push(cell);
            if let Some(dependents) = self.rdeps.get(&cell) {
                for dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(*dep);
                        }
                    }
                }
            }
        }

        // Check for cycle: if not all formula cells are in the order
        let formula_cells: HashSet<CellRef> = self.deps.keys().copied().collect();
        let ordered_set: HashSet<CellRef> = order.iter().copied().collect();
        let missing: Vec<CellRef> = formula_cells.difference(&ordered_set).copied().collect();
        if !missing.is_empty() {
            return Err(missing[0]);
        }

        Ok(order)
    }

    /// Given a changed cell, return all cells that need recalculation (in evaluation order).
    pub fn cells_to_recalculate(&self, changed: &CellRef) -> Vec<CellRef> {
        let mut visited = HashSet::new();
        let mut to_visit = VecDeque::new();
        to_visit.push_back(*changed);

        while let Some(cell) = to_visit.pop_front() {
            if let Some(dependents) = self.rdeps.get(&cell) {
                for dep in dependents {
                    if visited.insert(*dep) {
                        to_visit.push_back(*dep);
                    }
                }
            }
        }

        // Return in topological order (filter the full order to just the affected cells)
        match self.topological_order() {
            Ok(full_order) => full_order
                .into_iter()
                .filter(|c| visited.contains(c))
                .collect(),
            Err(_) => visited.into_iter().collect(),
        }
    }
}

/// Extract all cell references from a formula string (for dependency tracking).
fn extract_references(formula: &str) -> Vec<CellRef> {
    let tokens = match tokenize(formula) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let mut refs = Vec::new();
    for token in &tokens {
        match token {
            Token::CellRef(fref) => {
                refs.push(fref.to_cell_ref());
            }
            Token::RangeRef(frange) => {
                // Expand range to individual cell refs
                let range = frange.to_cell_range();
                for row in range.start.row..=range.end.row {
                    for col in range.start.col..=range.end.col {
                        refs.push(CellRef::new(col, row));
                    }
                }
            }
            Token::SheetRef(_, inner) => match inner.as_ref() {
                Token::CellRef(fref) => {
                    refs.push(fref.to_cell_ref());
                }
                Token::RangeRef(frange) => {
                    let range = frange.to_cell_range();
                    for row in range.start.row..=range.end.row {
                        for col in range.start.col..=range.end.col {
                            refs.push(CellRef::new(col, row));
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    refs
}

// ─── Workbook SheetLookup implementation ────────────────────

/// Provides cross-sheet reference resolution for a workbook.
pub struct WorkbookContext<'a> {
    /// Reference to all sheets in the workbook.
    pub sheets: &'a [crate::model::Sheet],
}

impl<'a> SheetLookup for WorkbookContext<'a> {
    fn get_sheet(&self, name: &str) -> Option<&dyn CellLookup> {
        self.sheets
            .iter()
            .find(|s| s.name == name)
            .map(|s| s as &dyn CellLookup)
    }
}

// ─── Sheet CellLookup implementation ───────────────────────

impl CellLookup for crate::model::Sheet {
    fn get_value(&self, cell: &CellRef) -> CellValue {
        self.cells
            .get(cell)
            .map(|c| c.value.clone())
            .unwrap_or(CellValue::Empty)
    }

    fn get_range_values(&self, range: &CellRange) -> Vec<CellValue> {
        let mut values = Vec::new();
        for row in range.start.row..=range.end.row {
            for col in range.start.col..=range.end.col {
                values.push(self.get_value(&CellRef::new(col, row)));
            }
        }
        values
    }
}

// ─── Sheet recalculate ─────────────────────────────────────

impl crate::model::Sheet {
    /// Recalculate all formula cells in dependency order.
    ///
    /// Cells with circular references will be set to `#REF!`.
    pub fn recalculate(&mut self) {
        let graph = DependencyGraph::build(self);
        match graph.topological_order() {
            Ok(order) => {
                for cell_ref in order {
                    // Read formula
                    let formula_str = match self.cells.get(&cell_ref) {
                        Some(cell) => match &cell.formula {
                            Some(f) => f.clone(),
                            None => continue,
                        },
                        None => continue,
                    };
                    let expr = match parse_formula(&formula_str) {
                        Ok(e) => e,
                        Err(_) => {
                            if let Some(cell) = self.cells.get_mut(&cell_ref) {
                                cell.value = CellValue::Error(CellError::Name);
                            }
                            continue;
                        }
                    };
                    // Evaluate with current sheet as context (self is &mut, but we need &)
                    // We must read-only evaluate, then write. Since we're iterating in topo order,
                    // all dependencies are already computed.
                    // We need a temporary immutable borrow workaround:
                    let value = {
                        // Build a snapshot lookup for just this evaluation
                        let lookup = SheetSnapshot { cells: &self.cells };
                        FormulaEngine::evaluate(&expr, &lookup)
                    };
                    if let Some(cell) = self.cells.get_mut(&cell_ref) {
                        cell.value = value;
                    }
                }
            }
            Err(cycle_cell) => {
                // Mark cells in cycles as #REF!
                // Find all cells that couldn't be ordered
                let formula_cells: HashSet<CellRef> = graph.deps.keys().copied().collect();
                // Try to identify which ones are in the cycle
                // Simple approach: mark all unresolvable cells
                let _ = cycle_cell; // We know there's a cycle
                                    // Re-run and mark problematic cells
                let ordered: HashSet<CellRef> = match graph.topological_order() {
                    Ok(o) => o.into_iter().collect(),
                    Err(_) => HashSet::new(),
                };
                for cell_ref in &formula_cells {
                    if !ordered.contains(cell_ref) {
                        if let Some(cell) = self.cells.get_mut(cell_ref) {
                            cell.value = CellValue::Error(CellError::Ref);
                        }
                    }
                }
            }
        }
    }
}

/// A read-only snapshot of cells for formula evaluation during recalculation.
struct SheetSnapshot<'a> {
    cells: &'a std::collections::BTreeMap<CellRef, crate::model::Cell>,
}

impl<'a> CellLookup for SheetSnapshot<'a> {
    fn get_value(&self, cell: &CellRef) -> CellValue {
        self.cells
            .get(cell)
            .map(|c| c.value.clone())
            .unwrap_or(CellValue::Empty)
    }

    fn get_range_values(&self, range: &CellRange) -> Vec<CellValue> {
        let mut values = Vec::new();
        for row in range.start.row..=range.end.row {
            for col in range.start.col..=range.end.col {
                values.push(self.get_value(&CellRef::new(col, row)));
            }
        }
        values
    }
}

// ─── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CellError, CellValue, Sheet};
    use std::collections::BTreeMap;

    /// Simple in-memory cell lookup for tests.
    struct TestCtx {
        cells: BTreeMap<CellRef, CellValue>,
    }

    impl TestCtx {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
            }
        }

        fn set(&mut self, col: u32, row: u32, val: CellValue) {
            self.cells.insert(CellRef::new(col, row), val);
        }
    }

    impl CellLookup for TestCtx {
        fn get_value(&self, cell: &CellRef) -> CellValue {
            self.cells.get(cell).cloned().unwrap_or(CellValue::Empty)
        }

        fn get_range_values(&self, range: &CellRange) -> Vec<CellValue> {
            let mut vals = Vec::new();
            for row in range.start.row..=range.end.row {
                for col in range.start.col..=range.end.col {
                    vals.push(self.get_value(&CellRef::new(col, row)));
                }
            }
            vals
        }
    }

    fn eval(formula: &str, ctx: &dyn CellLookup) -> CellValue {
        let expr = parse_formula(formula).expect("parse failed");
        FormulaEngine::evaluate(&expr, ctx)
    }

    // ─── Tokenizer Tests ───

    #[test]
    fn tokenize_number() {
        let tokens = tokenize("123").unwrap();
        assert_eq!(tokens, vec![Token::Number(123.0)]);
    }

    #[test]
    fn tokenize_float() {
        let tokens = tokenize("45.67").unwrap();
        assert_eq!(tokens, vec![Token::Number(45.67)]);
    }

    #[test]
    fn tokenize_leading_dot() {
        let tokens = tokenize(".5").unwrap();
        assert_eq!(tokens, vec![Token::Number(0.5)]);
    }

    #[test]
    fn tokenize_scientific() {
        let tokens = tokenize("1E10").unwrap();
        assert_eq!(tokens, vec![Token::Number(1e10)]);
    }

    #[test]
    fn tokenize_string() {
        let tokens = tokenize(r#""hello""#).unwrap();
        assert_eq!(tokens, vec![Token::String("hello".into())]);
    }

    #[test]
    fn tokenize_string_with_quotes() {
        // In Excel formulas, "" inside a string is an escaped double-quote.
        // So "say ""hi""  " means the string: say "hi"  (with trailing spaces)
        let tokens = tokenize(r#""say ""hi""  "#).unwrap();
        assert_eq!(tokens, vec![Token::String("say \"hi\"  ".into())]);
    }

    #[test]
    fn tokenize_bool() {
        let tokens = tokenize("TRUE").unwrap();
        assert_eq!(tokens, vec![Token::Bool(true)]);
        let tokens = tokenize("FALSE").unwrap();
        assert_eq!(tokens, vec![Token::Bool(false)]);
    }

    #[test]
    fn tokenize_cell_ref() {
        let tokens = tokenize("A1").unwrap();
        assert_eq!(
            tokens,
            vec![Token::CellRef(FormulaRef::new(0, 0, false, false))]
        );
    }

    #[test]
    fn tokenize_absolute_ref() {
        let tokens = tokenize("$A$1").unwrap();
        assert_eq!(
            tokens,
            vec![Token::CellRef(FormulaRef::new(0, 0, true, true))]
        );
    }

    #[test]
    fn tokenize_mixed_ref() {
        let tokens = tokenize("$A1").unwrap();
        assert_eq!(
            tokens,
            vec![Token::CellRef(FormulaRef::new(0, 0, true, false))]
        );
        let tokens = tokenize("A$1").unwrap();
        assert_eq!(
            tokens,
            vec![Token::CellRef(FormulaRef::new(0, 0, false, true))]
        );
    }

    #[test]
    fn tokenize_range() {
        let tokens = tokenize("A1:B10").unwrap();
        assert_eq!(
            tokens,
            vec![Token::RangeRef(FormulaRange {
                start: FormulaRef::new(0, 0, false, false),
                end: FormulaRef::new(1, 9, false, false),
            })]
        );
    }

    #[test]
    fn tokenize_sheet_ref() {
        let tokens = tokenize("Sheet1!A1").unwrap();
        assert_eq!(
            tokens,
            vec![Token::SheetRef(
                "Sheet1".into(),
                Box::new(Token::CellRef(FormulaRef::new(0, 0, false, false)))
            )]
        );
    }

    #[test]
    fn tokenize_function() {
        let tokens = tokenize("SUM(A1:A10)").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], Token::Function("SUM".into()));
        assert_eq!(tokens[1], Token::OpenParen);
        assert!(matches!(tokens[2], Token::RangeRef(_)));
        assert_eq!(tokens[3], Token::CloseParen);
    }

    #[test]
    fn tokenize_operators() {
        let tokens = tokenize("1+2*3").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Number(1.0),
                Token::Operator(Op::Add),
                Token::Number(2.0),
                Token::Operator(Op::Mul),
                Token::Number(3.0),
            ]
        );
    }

    #[test]
    fn tokenize_comparison_operators() {
        let tokens = tokenize("A1<>B1").unwrap();
        assert!(tokens.contains(&Token::Operator(Op::Ne)));
        let tokens = tokenize("A1<=B1").unwrap();
        assert!(tokens.contains(&Token::Operator(Op::Le)));
        let tokens = tokenize("A1>=B1").unwrap();
        assert!(tokens.contains(&Token::Operator(Op::Ge)));
    }

    #[test]
    fn tokenize_error_literal() {
        let tokens = tokenize("#DIV/0!").unwrap();
        assert_eq!(tokens, vec![Token::Error(CellError::DivZero)]);
        let tokens = tokenize("#N/A").unwrap();
        assert_eq!(tokens, vec![Token::Error(CellError::NA)]);
    }

    // ─── Parser Tests ───

    #[test]
    fn parse_number() {
        let expr = parse_formula("42").unwrap();
        assert_eq!(expr, Expr::Literal(CellValue::Number(42.0)));
    }

    #[test]
    fn parse_binary_ops() {
        let expr = parse_formula("1+2*3").unwrap();
        // Should be 1+(2*3) due to precedence
        match expr {
            Expr::BinaryOp(Op::Add, left, right) => {
                assert_eq!(*left, Expr::Literal(CellValue::Number(1.0)));
                match *right {
                    Expr::BinaryOp(Op::Mul, ref l, ref r) => {
                        assert_eq!(**l, Expr::Literal(CellValue::Number(2.0)));
                        assert_eq!(**r, Expr::Literal(CellValue::Number(3.0)));
                    }
                    _ => panic!("Expected multiply"),
                }
            }
            _ => panic!("Expected add"),
        }
    }

    #[test]
    fn parse_unary_minus() {
        let expr = parse_formula("-5").unwrap();
        assert_eq!(
            expr,
            Expr::UnaryOp(Op::Sub, Box::new(Expr::Literal(CellValue::Number(5.0))))
        );
    }

    #[test]
    fn parse_percent() {
        let expr = parse_formula("50%").unwrap();
        assert_eq!(
            expr,
            Expr::Percent(Box::new(Expr::Literal(CellValue::Number(50.0))))
        );
    }

    #[test]
    fn parse_function_call() {
        let expr = parse_formula("SUM(1,2,3)").unwrap();
        match expr {
            Expr::FunctionCall(name, args) => {
                assert_eq!(name, "SUM");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn parse_nested_functions() {
        let expr = parse_formula("IF(A1>5,SUM(B1:B10),0)").unwrap();
        match expr {
            Expr::FunctionCall(name, args) => {
                assert_eq!(name, "IF");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected IF function"),
        }
    }

    #[test]
    fn parse_parenthesized() {
        let expr = parse_formula("(1+2)*3").unwrap();
        match expr {
            Expr::BinaryOp(Op::Mul, _, _) => {}
            _ => panic!("Expected multiply at top level"),
        }
    }

    #[test]
    fn parse_power_right_associative() {
        let expr = parse_formula("2^3^4").unwrap();
        // Should be 2^(3^4) — right-associative
        match expr {
            Expr::BinaryOp(Op::Pow, _, right) => match *right {
                Expr::BinaryOp(Op::Pow, _, _) => {}
                _ => panic!("Expected power on right"),
            },
            _ => panic!("Expected power"),
        }
    }

    // ─── Evaluator Tests ───

    #[test]
    fn eval_literal() {
        let ctx = TestCtx::new();
        assert_eq!(eval("42", &ctx), CellValue::Number(42.0));
        assert_eq!(eval(r#""hello""#, &ctx), CellValue::Text("hello".into()));
        assert_eq!(eval("TRUE", &ctx), CellValue::Boolean(true));
    }

    #[test]
    fn eval_arithmetic() {
        let ctx = TestCtx::new();
        assert_eq!(eval("2+3", &ctx), CellValue::Number(5.0));
        assert_eq!(eval("10-4", &ctx), CellValue::Number(6.0));
        assert_eq!(eval("3*7", &ctx), CellValue::Number(21.0));
        assert_eq!(eval("15/3", &ctx), CellValue::Number(5.0));
        assert_eq!(eval("2^10", &ctx), CellValue::Number(1024.0));
    }

    #[test]
    fn eval_operator_precedence() {
        let ctx = TestCtx::new();
        assert_eq!(eval("2+3*4", &ctx), CellValue::Number(14.0));
        assert_eq!(eval("(2+3)*4", &ctx), CellValue::Number(20.0));
    }

    #[test]
    fn eval_div_zero() {
        let ctx = TestCtx::new();
        assert_eq!(eval("1/0", &ctx), CellValue::Error(CellError::DivZero));
    }

    #[test]
    fn eval_concat() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#""Hello"&" "&"World""#, &ctx),
            CellValue::Text("Hello World".into())
        );
    }

    #[test]
    fn eval_comparison() {
        let ctx = TestCtx::new();
        assert_eq!(eval("1=1", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("1=2", &ctx), CellValue::Boolean(false));
        assert_eq!(eval("1<2", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("2>1", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("1<>2", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("1<=1", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("1>=2", &ctx), CellValue::Boolean(false));
    }

    #[test]
    fn eval_percent() {
        let ctx = TestCtx::new();
        assert_eq!(eval("50%", &ctx), CellValue::Number(0.5));
    }

    #[test]
    fn eval_unary_minus() {
        let ctx = TestCtx::new();
        assert_eq!(eval("-5", &ctx), CellValue::Number(-5.0));
        assert_eq!(eval("-(-3)", &ctx), CellValue::Number(3.0));
    }

    #[test]
    fn eval_cell_ref() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(42.0)); // A1
        ctx.set(1, 0, CellValue::Number(8.0)); // B1
        assert_eq!(eval("A1+B1", &ctx), CellValue::Number(50.0));
    }

    #[test]
    fn eval_empty_cell() {
        let ctx = TestCtx::new();
        // Empty cell coerces to 0 for arithmetic
        assert_eq!(eval("A1+1", &ctx), CellValue::Number(1.0));
    }

    // ─── P0 Function Tests ───

    #[test]
    fn eval_sum() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(0, 1, CellValue::Number(20.0));
        ctx.set(0, 2, CellValue::Number(30.0));
        assert_eq!(eval("SUM(A1:A3)", &ctx), CellValue::Number(60.0));
    }

    #[test]
    fn eval_sum_multiple_args() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(1, 0, CellValue::Number(20.0));
        assert_eq!(eval("SUM(A1,B1,5)", &ctx), CellValue::Number(35.0));
    }

    #[test]
    fn eval_average() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(0, 1, CellValue::Number(20.0));
        ctx.set(0, 2, CellValue::Number(30.0));
        assert_eq!(eval("AVERAGE(A1:A3)", &ctx), CellValue::Number(20.0));
    }

    #[test]
    fn eval_average_empty_range() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval("AVERAGE(A1:A3)", &ctx),
            CellValue::Error(CellError::DivZero)
        );
    }

    #[test]
    fn eval_min_max() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(5.0));
        ctx.set(0, 1, CellValue::Number(15.0));
        ctx.set(0, 2, CellValue::Number(3.0));
        assert_eq!(eval("MIN(A1:A3)", &ctx), CellValue::Number(3.0));
        assert_eq!(eval("MAX(A1:A3)", &ctx), CellValue::Number(15.0));
    }

    #[test]
    fn eval_count() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(0, 1, CellValue::Text("hello".into()));
        ctx.set(0, 2, CellValue::Number(3.0));
        assert_eq!(eval("COUNT(A1:A3)", &ctx), CellValue::Number(2.0));
    }

    #[test]
    fn eval_counta() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(0, 1, CellValue::Text("hello".into()));
        // A3 is empty
        assert_eq!(eval("COUNTA(A1:A3)", &ctx), CellValue::Number(2.0));
    }

    #[test]
    fn eval_if_true() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"IF(1>0,"yes","no")"#, &ctx),
            CellValue::Text("yes".into())
        );
    }

    #[test]
    fn eval_if_false() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"IF(1>2,"yes","no")"#, &ctx),
            CellValue::Text("no".into())
        );
    }

    #[test]
    fn eval_and() {
        let ctx = TestCtx::new();
        assert_eq!(eval("AND(TRUE,TRUE)", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("AND(TRUE,FALSE)", &ctx), CellValue::Boolean(false));
    }

    #[test]
    fn eval_or() {
        let ctx = TestCtx::new();
        assert_eq!(eval("OR(FALSE,TRUE)", &ctx), CellValue::Boolean(true));
        assert_eq!(eval("OR(FALSE,FALSE)", &ctx), CellValue::Boolean(false));
    }

    #[test]
    fn eval_not() {
        let ctx = TestCtx::new();
        assert_eq!(eval("NOT(TRUE)", &ctx), CellValue::Boolean(false));
        assert_eq!(eval("NOT(FALSE)", &ctx), CellValue::Boolean(true));
    }

    #[test]
    fn eval_iferror() {
        let ctx = TestCtx::new();
        assert_eq!(eval("IFERROR(1/0,0)", &ctx), CellValue::Number(0.0));
        assert_eq!(eval("IFERROR(42,0)", &ctx), CellValue::Number(42.0));
    }

    // ─── P1 Lookup Function Tests ───

    #[test]
    fn eval_vlookup_exact() {
        let mut ctx = TestCtx::new();
        // A1:B3 lookup table
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(1, 0, CellValue::Text("one".into()));
        ctx.set(0, 1, CellValue::Number(2.0));
        ctx.set(1, 1, CellValue::Text("two".into()));
        ctx.set(0, 2, CellValue::Number(3.0));
        ctx.set(1, 2, CellValue::Text("three".into()));

        assert_eq!(
            eval("VLOOKUP(2,A1:B3,2,FALSE)", &ctx),
            CellValue::Text("two".into())
        );
    }

    #[test]
    fn eval_vlookup_not_found() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(1, 0, CellValue::Text("one".into()));

        assert_eq!(
            eval("VLOOKUP(99,A1:B1,2,FALSE)", &ctx),
            CellValue::Error(CellError::NA)
        );
    }

    #[test]
    fn eval_hlookup_exact() {
        let mut ctx = TestCtx::new();
        // A1:C2 — first row is keys, second row is values
        ctx.set(0, 0, CellValue::Text("x".into()));
        ctx.set(1, 0, CellValue::Text("y".into()));
        ctx.set(2, 0, CellValue::Text("z".into()));
        ctx.set(0, 1, CellValue::Number(10.0));
        ctx.set(1, 1, CellValue::Number(20.0));
        ctx.set(2, 1, CellValue::Number(30.0));

        assert_eq!(
            eval(r#"HLOOKUP("y",A1:C2,2,FALSE)"#, &ctx),
            CellValue::Number(20.0)
        );
    }

    #[test]
    fn eval_index() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(1, 0, CellValue::Number(2.0));
        ctx.set(0, 1, CellValue::Number(3.0));
        ctx.set(1, 1, CellValue::Number(4.0));

        assert_eq!(eval("INDEX(A1:B2,2,1)", &ctx), CellValue::Number(3.0));
        assert_eq!(eval("INDEX(A1:B2,1,2)", &ctx), CellValue::Number(2.0));
    }

    #[test]
    fn eval_match_exact() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(0, 1, CellValue::Number(20.0));
        ctx.set(0, 2, CellValue::Number(30.0));

        assert_eq!(eval("MATCH(20,A1:A3,0)", &ctx), CellValue::Number(2.0));
    }

    // ─── P1 String Function Tests ───

    #[test]
    fn eval_left() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"LEFT("Hello",3)"#, &ctx),
            CellValue::Text("Hel".into())
        );
        assert_eq!(eval(r#"LEFT("Hi")"#, &ctx), CellValue::Text("H".into()));
    }

    #[test]
    fn eval_right() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"RIGHT("Hello",3)"#, &ctx),
            CellValue::Text("llo".into())
        );
    }

    #[test]
    fn eval_mid() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"MID("Hello World",7,5)"#, &ctx),
            CellValue::Text("World".into())
        );
    }

    #[test]
    fn eval_len() {
        let ctx = TestCtx::new();
        assert_eq!(eval(r#"LEN("Hello")"#, &ctx), CellValue::Number(5.0));
    }

    #[test]
    fn eval_trim() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"TRIM("  hello   world  ")"#, &ctx),
            CellValue::Text("hello world".into())
        );
    }

    #[test]
    fn eval_concatenate() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"CONCATENATE("Hello"," ","World")"#, &ctx),
            CellValue::Text("Hello World".into())
        );
    }

    #[test]
    fn eval_upper_lower() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval(r#"UPPER("hello")"#, &ctx),
            CellValue::Text("HELLO".into())
        );
        assert_eq!(
            eval(r#"LOWER("HELLO")"#, &ctx),
            CellValue::Text("hello".into())
        );
    }

    // ─── P1 Math Function Tests ───

    #[test]
    fn eval_round() {
        let ctx = TestCtx::new();
        assert_eq!(eval("ROUND(3.14159,2)", &ctx), CellValue::Number(3.14));
        assert_eq!(eval("ROUND(1234,-2)", &ctx), CellValue::Number(1200.0));
    }

    #[test]
    fn eval_abs() {
        let ctx = TestCtx::new();
        assert_eq!(eval("ABS(-5)", &ctx), CellValue::Number(5.0));
        assert_eq!(eval("ABS(5)", &ctx), CellValue::Number(5.0));
    }

    #[test]
    fn eval_int() {
        let ctx = TestCtx::new();
        assert_eq!(eval("INT(7.8)", &ctx), CellValue::Number(7.0));
        assert_eq!(eval("INT(-7.2)", &ctx), CellValue::Number(-8.0));
    }

    #[test]
    fn eval_mod() {
        let ctx = TestCtx::new();
        assert_eq!(eval("MOD(10,3)", &ctx), CellValue::Number(1.0));
        assert_eq!(
            eval("MOD(10,0)", &ctx),
            CellValue::Error(CellError::DivZero)
        );
    }

    #[test]
    fn eval_power() {
        let ctx = TestCtx::new();
        assert_eq!(eval("POWER(2,10)", &ctx), CellValue::Number(1024.0));
    }

    #[test]
    fn eval_sqrt() {
        let ctx = TestCtx::new();
        assert_eq!(eval("SQRT(16)", &ctx), CellValue::Number(4.0));
        assert_eq!(eval("SQRT(-1)", &ctx), CellValue::Error(CellError::Num));
    }

    // ─── P1 Date Function Tests ───

    #[test]
    fn eval_date() {
        let ctx = TestCtx::new();
        // DATE(2024,1,1) should be serial for Jan 1, 2024
        let result = eval("DATE(2024,1,1)", &ctx);
        match result {
            CellValue::Number(n) => {
                assert!(n > 40000.0, "Serial should be > 40000 for 2024");
                // Verify round-trip
                let (y, m, d) = serial_to_date(n);
                assert_eq!((y, m, d), (2024, 1, 1));
            }
            _ => panic!("Expected number, got {result:?}"),
        }
    }

    #[test]
    fn eval_year_month_day() {
        let ctx = TestCtx::new();
        // Use DATE to get a serial, then extract parts
        let serial = date_to_serial(2024, 3, 15);
        let mut ctx2 = TestCtx::new();
        ctx2.set(0, 0, CellValue::Number(serial));

        assert_eq!(eval("YEAR(A1)", &ctx2), CellValue::Number(2024.0));
        assert_eq!(eval("MONTH(A1)", &ctx2), CellValue::Number(3.0));
        assert_eq!(eval("DAY(A1)", &ctx2), CellValue::Number(15.0));

        // Also test NOW/TODAY return something reasonable
        let now_val = eval("NOW()", &ctx);
        match now_val {
            CellValue::Number(n) => assert!(n > 40000.0),
            _ => panic!("NOW() should return a number"),
        }
    }

    #[test]
    fn eval_date_roundtrip_various() {
        // Test several known dates with their expected Excel serial numbers
        let cases = vec![
            (1900, 1, 1, 1.0),     // Day 1
            (1900, 2, 28, 59.0),   // Last real day before the bug
            (1900, 3, 1, 61.0),    // First day after the phantom Feb 29
            (2000, 1, 1, 36526.0), // Y2K
            (1970, 1, 1, 25569.0), // Unix epoch
            (2024, 1, 1, 45292.0), // Recent date
        ];
        for (y, m, d, expected_serial) in &cases {
            let serial = date_to_serial(*y, *m, *d);
            assert_eq!(
                serial, *expected_serial,
                "date_to_serial({y},{m},{d}) = {serial}, expected {expected_serial}"
            );
            let (ry, rm, rd) = serial_to_date(serial);
            assert_eq!(
                (ry, rm, rd),
                (*y, *m, *d),
                "Round-trip failed for {y}-{m}-{d} (serial={serial})"
            );
        }
    }

    // ─── Dependency Graph Tests ───

    #[test]
    fn dependency_graph_simple() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(10.0)); // A1 = 10
        sheet.set(0, 1, CellValue::Number(20.0)); // A2 = 20
        sheet.set_formula(0, 2, "A1+A2", CellValue::Number(0.0)); // A3 = A1+A2

        let graph = DependencyGraph::build(&sheet);
        let order = graph.topological_order().unwrap();
        // A3 should be the only formula cell
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], CellRef::new(0, 2));
    }

    #[test]
    fn dependency_graph_chain() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(1.0)); // A1 = 1
        sheet.set_formula(0, 1, "A1+1", CellValue::Number(0.0)); // A2 = A1+1
        sheet.set_formula(0, 2, "A2+1", CellValue::Number(0.0)); // A3 = A2+1

        let graph = DependencyGraph::build(&sheet);
        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 2);
        // A2 must come before A3
        let pos_a2 = order.iter().position(|c| *c == CellRef::new(0, 1)).unwrap();
        let pos_a3 = order.iter().position(|c| *c == CellRef::new(0, 2)).unwrap();
        assert!(pos_a2 < pos_a3);
    }

    #[test]
    fn dependency_graph_circular() {
        let mut sheet = Sheet::default();
        sheet.set_formula(0, 0, "B1", CellValue::Number(0.0)); // A1 = B1
        sheet.set_formula(1, 0, "A1", CellValue::Number(0.0)); // B1 = A1

        let graph = DependencyGraph::build(&sheet);
        let result = graph.topological_order();
        assert!(result.is_err(), "Should detect circular reference");
    }

    #[test]
    fn dependency_graph_cells_to_recalculate() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(1.0)); // A1
        sheet.set_formula(0, 1, "A1*2", CellValue::Number(0.0)); // A2 = A1*2
        sheet.set_formula(0, 2, "A2+10", CellValue::Number(0.0)); // A3 = A2+10

        let graph = DependencyGraph::build(&sheet);
        let to_recalc = graph.cells_to_recalculate(&CellRef::new(0, 0));
        // Changing A1 should require recalculating A2 and A3
        assert!(to_recalc.contains(&CellRef::new(0, 1)));
        assert!(to_recalc.contains(&CellRef::new(0, 2)));
    }

    // ─── Sheet Recalculate Tests ───

    #[test]
    fn sheet_recalculate_simple() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(10.0));
        sheet.set(0, 1, CellValue::Number(20.0));
        sheet.set_formula(0, 2, "A1+A2", CellValue::Number(0.0));

        sheet.recalculate();

        assert_eq!(sheet.get(0, 2).unwrap().value, CellValue::Number(30.0));
    }

    #[test]
    fn sheet_recalculate_chain() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(5.0));
        sheet.set_formula(0, 1, "A1*2", CellValue::Number(0.0)); // A2 = 10
        sheet.set_formula(0, 2, "A2+A1", CellValue::Number(0.0)); // A3 = 15

        sheet.recalculate();

        assert_eq!(sheet.get(0, 1).unwrap().value, CellValue::Number(10.0));
        assert_eq!(sheet.get(0, 2).unwrap().value, CellValue::Number(15.0));
    }

    #[test]
    fn sheet_recalculate_sum_range() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(1.0));
        sheet.set(0, 1, CellValue::Number(2.0));
        sheet.set(0, 2, CellValue::Number(3.0));
        sheet.set(0, 3, CellValue::Number(4.0));
        sheet.set_formula(0, 4, "SUM(A1:A4)", CellValue::Number(0.0));

        sheet.recalculate();

        assert_eq!(sheet.get(0, 4).unwrap().value, CellValue::Number(10.0));
    }

    #[test]
    fn sheet_recalculate_with_if() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(100.0));
        sheet.set_formula(0, 1, r#"IF(A1>50,"big","small")"#, CellValue::Empty);

        sheet.recalculate();

        assert_eq!(
            sheet.get(0, 1).unwrap().value,
            CellValue::Text("big".into())
        );
    }

    #[test]
    fn sheet_recalculate_circular_ref() {
        let mut sheet = Sheet::default();
        sheet.set_formula(0, 0, "B1+1", CellValue::Number(0.0));
        sheet.set_formula(1, 0, "A1+1", CellValue::Number(0.0));

        sheet.recalculate();

        // Circular cells should be marked as #REF!
        assert_eq!(
            sheet.get(0, 0).unwrap().value,
            CellValue::Error(CellError::Ref)
        );
        assert_eq!(
            sheet.get(1, 0).unwrap().value,
            CellValue::Error(CellError::Ref)
        );
    }

    #[test]
    fn eval_complex_formula() {
        let mut ctx = TestCtx::new();
        // Build a small table
        for i in 0..5 {
            ctx.set(0, i, CellValue::Number((i as f64 + 1.0) * 10.0));
        }
        // SUM + arithmetic
        assert_eq!(
            eval("SUM(A1:A5)*2+100", &ctx),
            CellValue::Number(400.0) // (10+20+30+40+50)*2+100 = 400
        );
    }

    #[test]
    fn eval_nested_if_and_or() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(85.0));
        // IF(AND(A1>=80,A1<90),"B","other")
        assert_eq!(
            eval(r#"IF(AND(A1>=80,A1<90),"B","other")"#, &ctx),
            CellValue::Text("B".into())
        );
    }

    #[test]
    fn eval_iferror_with_vlookup() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(1, 0, CellValue::Text("found".into()));
        // IFERROR with a lookup that fails
        assert_eq!(
            eval(r#"IFERROR(VLOOKUP(99,A1:B1,2,FALSE),"not found")"#, &ctx),
            CellValue::Text("not found".into())
        );
    }

    #[test]
    fn eval_unknown_function() {
        let ctx = TestCtx::new();
        assert_eq!(
            eval("NONEXISTENT(1)", &ctx),
            CellValue::Error(CellError::Name)
        );
    }

    #[test]
    fn eval_error_propagation() {
        let ctx = TestCtx::new();
        // Error in left operand should propagate
        assert_eq!(eval("(1/0)+5", &ctx), CellValue::Error(CellError::DivZero));
    }

    #[test]
    fn extract_references_test() {
        let refs = extract_references("A1+B2+SUM(C1:C10)");
        assert!(refs.contains(&CellRef::new(0, 0))); // A1
        assert!(refs.contains(&CellRef::new(1, 1))); // B2
                                                     // C1:C10 should expand to C1..C10
        assert!(refs.contains(&CellRef::new(2, 0))); // C1
        assert!(refs.contains(&CellRef::new(2, 9))); // C10
        assert_eq!(refs.len(), 12); // A1 + B2 + C1..C10
    }

    #[test]
    fn sheet_recalculate_average_and_count() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Number(10.0));
        sheet.set(0, 1, CellValue::Number(20.0));
        sheet.set(0, 2, CellValue::Text("skip".into()));
        sheet.set(0, 3, CellValue::Number(30.0));
        sheet.set_formula(1, 0, "AVERAGE(A1:A4)", CellValue::Number(0.0));
        sheet.set_formula(1, 1, "COUNT(A1:A4)", CellValue::Number(0.0));

        sheet.recalculate();

        assert_eq!(
            sheet.get(1, 0).unwrap().value,
            CellValue::Number(20.0) // (10+20+30)/3
        );
        assert_eq!(sheet.get(1, 1).unwrap().value, CellValue::Number(3.0));
    }

    // ─── COUNTIF / SUMIF / AVERAGEIF Tests ───

    #[test]
    fn eval_countif_numeric() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(0, 1, CellValue::Number(20.0));
        ctx.set(0, 2, CellValue::Number(30.0));
        ctx.set(0, 3, CellValue::Number(5.0));
        ctx.set(0, 4, CellValue::Number(15.0));

        // Count cells > 10
        assert_eq!(
            eval(r#"COUNTIF(A1:A5,">10")"#, &ctx),
            CellValue::Number(3.0) // 20, 30, 15
        );

        // Count cells = 10
        assert_eq!(eval("COUNTIF(A1:A5,10)", &ctx), CellValue::Number(1.0));

        // Count cells >= 15
        assert_eq!(
            eval(r#"COUNTIF(A1:A5,">=15")"#, &ctx),
            CellValue::Number(3.0) // 20, 30, 15
        );
    }

    #[test]
    fn eval_countif_text() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Text("apple".into()));
        ctx.set(0, 1, CellValue::Text("banana".into()));
        ctx.set(0, 2, CellValue::Text("apple".into()));
        ctx.set(0, 3, CellValue::Text("cherry".into()));

        // Exact text match (case-insensitive)
        assert_eq!(
            eval(r#"COUNTIF(A1:A4,"apple")"#, &ctx),
            CellValue::Number(2.0)
        );
    }

    #[test]
    fn eval_countif_wildcard() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Text("apple".into()));
        ctx.set(0, 1, CellValue::Text("application".into()));
        ctx.set(0, 2, CellValue::Text("banana".into()));
        ctx.set(0, 3, CellValue::Text("appetizer".into()));

        // Wildcard match: starts with "app"
        assert_eq!(
            eval(r#"COUNTIF(A1:A4,"app*")"#, &ctx),
            CellValue::Number(3.0) // apple, application, appetizer
        );
    }

    #[test]
    fn eval_sumif_basic() {
        let mut ctx = TestCtx::new();
        // Criteria range (categories)
        ctx.set(0, 0, CellValue::Text("fruit".into()));
        ctx.set(0, 1, CellValue::Text("veg".into()));
        ctx.set(0, 2, CellValue::Text("fruit".into()));
        ctx.set(0, 3, CellValue::Text("veg".into()));
        // Sum range (amounts)
        ctx.set(1, 0, CellValue::Number(10.0));
        ctx.set(1, 1, CellValue::Number(20.0));
        ctx.set(1, 2, CellValue::Number(30.0));
        ctx.set(1, 3, CellValue::Number(40.0));

        // Sum amounts where category = "fruit"
        assert_eq!(
            eval(r#"SUMIF(A1:A4,"fruit",B1:B4)"#, &ctx),
            CellValue::Number(40.0) // 10 + 30
        );
    }

    #[test]
    fn eval_sumif_numeric_criteria() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(0, 1, CellValue::Number(20.0));
        ctx.set(0, 2, CellValue::Number(30.0));
        ctx.set(0, 3, CellValue::Number(5.0));

        // Sum cells > 10 (no separate sum_range, so criteria_range = sum_range)
        assert_eq!(
            eval(r#"SUMIF(A1:A4,">10")"#, &ctx),
            CellValue::Number(50.0) // 20 + 30
        );
    }

    #[test]
    fn eval_averageif_basic() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(10.0));
        ctx.set(0, 1, CellValue::Number(20.0));
        ctx.set(0, 2, CellValue::Number(30.0));
        ctx.set(0, 3, CellValue::Number(5.0));
        ctx.set(0, 4, CellValue::Number(15.0));

        // Average of cells > 10
        assert_eq!(
            eval(r#"AVERAGEIF(A1:A5,">10")"#, &ctx),
            // 20 + 30 + 15 = 65 / 3 = 21.666...
            CellValue::Number(65.0 / 3.0)
        );
    }

    #[test]
    fn eval_averageif_no_match() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(1.0));
        ctx.set(0, 1, CellValue::Number(2.0));

        // No cells > 100 => #DIV/0!
        assert_eq!(
            eval(r#"AVERAGEIF(A1:A2,">100")"#, &ctx),
            CellValue::Error(CellError::DivZero)
        );
    }

    #[test]
    fn eval_countif_not_equal() {
        let mut ctx = TestCtx::new();
        ctx.set(0, 0, CellValue::Number(0.0));
        ctx.set(0, 1, CellValue::Number(5.0));
        ctx.set(0, 2, CellValue::Number(0.0));
        ctx.set(0, 3, CellValue::Number(10.0));

        // Count cells <> 0
        assert_eq!(
            eval(r#"COUNTIF(A1:A4,"<>0")"#, &ctx),
            CellValue::Number(2.0) // 5, 10
        );
    }

    #[test]
    fn wildcard_match_tests() {
        assert!(wildcard_match("hello", "hel*"));
        assert!(wildcard_match("hello", "*llo"));
        assert!(wildcard_match("hello", "h*o"));
        assert!(wildcard_match("hello", "h?llo"));
        assert!(!wildcard_match("hello", "h?lo"));
        assert!(wildcard_match("hello", "*"));
        assert!(wildcard_match("", "*"));
        assert!(!wildcard_match("", "?"));
        assert!(wildcard_match("abc", "a*c"));
        assert!(wildcard_match("abc", "a?c"));
    }

    // ─── Cross-Sheet Reference Tests ───

    /// Helper for multi-sheet evaluation: two contexts representing different sheets.
    struct TwoSheetLookup {
        sheet1: TestCtx,
        sheet2: TestCtx,
    }

    impl SheetLookup for TwoSheetLookup {
        fn get_sheet(&self, name: &str) -> Option<&dyn CellLookup> {
            match name {
                "Sheet1" => Some(&self.sheet1),
                "Sheet2" => Some(&self.sheet2),
                _ => None,
            }
        }
    }

    fn eval_with_sheets(
        formula: &str,
        ctx: &dyn CellLookup,
        sheets: &dyn SheetLookup,
    ) -> CellValue {
        let expr = parse_formula(formula).expect("parse failed");
        FormulaEngine::evaluate_with_sheets(&expr, ctx, Some(sheets))
    }

    #[test]
    fn cross_sheet_cell_ref() {
        let mut sheet1 = TestCtx::new();
        sheet1.set(0, 0, CellValue::Number(42.0));
        let sheet2 = TestCtx::new();

        let lookup = TwoSheetLookup {
            sheet1,
            sheet2: sheet2,
        };
        // Evaluate Sheet1!A1 from sheet2's perspective
        let ctx2 = &lookup.sheet2 as &dyn CellLookup;
        assert_eq!(
            eval_with_sheets("Sheet1!A1", ctx2, &lookup),
            CellValue::Number(42.0)
        );
    }

    #[test]
    fn cross_sheet_range_ref() {
        let mut sheet1 = TestCtx::new();
        sheet1.set(0, 0, CellValue::Number(10.0));
        sheet1.set(0, 1, CellValue::Number(20.0));
        sheet1.set(0, 2, CellValue::Number(30.0));
        let sheet2 = TestCtx::new();

        let lookup = TwoSheetLookup { sheet1, sheet2 };
        let ctx2 = &lookup.sheet2 as &dyn CellLookup;
        // SUM of cross-sheet range — the parser creates
        // SheetRef("Sheet1", Range(A1:A3)) which evaluate_with_sheets resolves
        // by switching the context to sheet1 for the inner expression.
        // However, SUM expects a Range arg that it collects values from using ctx,
        // so after the SheetRef is resolved, the range is evaluated on sheet1.
        assert_eq!(
            eval_with_sheets("SUM(Sheet1!A1:A3)", ctx2, &lookup),
            CellValue::Number(60.0)
        );
    }

    #[test]
    fn cross_sheet_nonexistent() {
        let sheet1 = TestCtx::new();
        let sheet2 = TestCtx::new();

        let lookup = TwoSheetLookup { sheet1, sheet2 };
        let ctx = &lookup.sheet1 as &dyn CellLookup;
        assert_eq!(
            eval_with_sheets("NoSuchSheet!A1", ctx, &lookup),
            CellValue::Error(CellError::Ref)
        );
    }

    #[test]
    fn sheet_lookup_trait_basic() {
        // Verify the WorkbookContext works with real Sheet objects
        let mut s1 = Sheet::default();
        s1.name = "Sales".to_string();
        s1.cells.insert(
            CellRef::new(0, 0),
            crate::model::Cell {
                value: CellValue::Number(100.0),
                formula: None,
                style_id: 0,
            },
        );

        let mut s2 = Sheet::default();
        s2.name = "Summary".to_string();

        let sheets = vec![s1, s2];
        let wb_ctx = WorkbookContext { sheets: &sheets };
        assert!(wb_ctx.get_sheet("Sales").is_some());
        assert!(wb_ctx.get_sheet("Summary").is_some());
        assert!(wb_ctx.get_sheet("Missing").is_none());

        // Evaluate a cross-sheet formula from Summary's perspective
        let summary = &sheets[1] as &dyn CellLookup;
        let expr = parse_formula("Sales!A1").unwrap();
        let result = FormulaEngine::evaluate_with_sheets(&expr, summary, Some(&wb_ctx));
        assert_eq!(result, CellValue::Number(100.0));
    }
}
