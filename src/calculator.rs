use std::f64::consts::{E, PI};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PieceKind {
    Number,
    Decimal,
    Exponent,
    Operator,
    OpenParen,
    CloseParen,
    FunctionStart,
    Constant,
    Postfix,
}

#[derive(Clone, Debug)]
struct Piece {
    internal: String,
    display: String,
    kind: PieceKind,
}

impl Piece {
    fn new(internal: impl Into<String>, display: impl Into<String>, kind: PieceKind) -> Self {
        Self {
            internal: internal.into(),
            display: display.into(),
            kind,
        }
    }
}

pub struct Calculator {
    pieces: Vec<Piece>,
    answer: String,
    evaluated_formula: String,
    just_evaluated: bool,
    pub scientific_mode: bool,
    pub second_mode: bool,
    pub radians_mode: bool,
    memory: f64,
    random_seed: u64,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            pieces: Vec::new(),
            answer: "0".to_string(),
            evaluated_formula: String::new(),
            just_evaluated: false,
            scientific_mode: false,
            second_mode: false,
            radians_mode: false,
            memory: 0.0,
            random_seed: 0x12d3_f4a5_6789_bcde,
        }
    }

    pub fn formula_text(&self) -> String {
        if self.just_evaluated {
            self.evaluated_formula.clone()
        } else {
            self.display_expression()
        }
    }

    pub fn answer_text(&self) -> &str {
        &self.answer
    }

    pub fn result_mode(&self) -> bool {
        self.just_evaluated
    }

    pub fn clear_label(&self) -> &'static str {
        if self.pieces.is_empty() && !self.just_evaluated {
            "AC"
        } else {
            "C"
        }
    }

    pub fn press(&mut self, input: &str) {
        match input {
            "mode-basic" => {
                self.scientific_mode = false;
                self.second_mode = false;
                return;
            }
            "mode-scientific" => {
                self.scientific_mode = true;
                return;
            }
            "second" => {
                self.second_mode = !self.second_mode;
                return;
            }
            "angle" => {
                self.radians_mode = !self.radians_mode;
                if self.just_evaluated {
                    self.evaluated_formula = self.autoclosed_display_expression();
                    self.answer = self
                        .evaluate_current()
                        .map(format_result)
                        .unwrap_or_else(|_| "Error".to_string());
                } else {
                    self.refresh();
                }
                return;
            }
            _ => {}
        }

        match input {
            "clear" => self.clear(),
            "backspace" => self.backspace(),
            "equals" => self.equals(),
            "sign" => self.toggle_sign(),
            "decimal" => self.push_decimal(),
            "ee" => self.push_exponent_marker(),
            "percent" => self.push_postfix("%", "%"),
            "plus" => {
                self.push_binary_operator("+", "+");
            }
            "minus" => {
                self.push_binary_operator("-", "−");
            }
            "multiply" => {
                self.push_binary_operator("*", "×");
            }
            "divide" => {
                self.push_binary_operator("/", "÷");
            }
            "open" => self.push_open_paren(),
            "close" => self.push_close_paren(),
            "mc" => self.memory = 0.0,
            "mplus" => {
                if let Some(value) = self.current_value() {
                    self.memory += value;
                }
            }
            "mminus" => {
                if let Some(value) = self.current_value() {
                    self.memory -= value;
                }
            }
            "mrecall" => self.insert_number_literal(format_number_internal(self.memory)),
            "square" => self.push_power_literal("2"),
            "cube" => self.push_power_literal("3"),
            "power" | "ypower" => {
                self.push_binary_operator("^", "^");
            }
            "yroot" => self.push_yroot_operator(),
            "logy" => self.push_logbase_operator(),
            "sqrt" => self.apply_unary_function("sqrt", "sqrt"),
            "cbrt" => self.apply_unary_function("cbrt", "cbrt"),
            "reciprocal" => self.wrap_or_start("1/(", "1÷("),
            "factorial" => self.push_postfix("!", "!"),
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh"
            | "asinh" | "acosh" | "atanh" | "ln" | "log10" | "log2" => {
                self.apply_unary_function(input, input)
            }
            "exp" => self.apply_power_prefix("econst", "e^"),
            "tenpower" => self.apply_power_prefix("10", "10^"),
            "twopower" => self.apply_power_prefix("2", "2^"),
            "constant-e" => self.insert_constant("econst", "e"),
            "pi" => self.insert_constant("pi", "π"),
            "rand" => {
                let value = self.next_random();
                self.insert_number_literal(format_number_internal(value));
            }
            digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
                self.push_digit(digit)
            }
            _ => {}
        }

        self.refresh();
    }

    fn clear(&mut self) {
        self.pieces.clear();
        self.just_evaluated = false;
        self.evaluated_formula.clear();
        self.answer = "0".to_string();
    }

    fn backspace(&mut self) {
        if self.just_evaluated {
            self.clear();
            return;
        }
        self.pieces.pop();
    }

    fn equals(&mut self) {
        if self.pieces.is_empty() {
            return;
        }

        let formula = self.autoclosed_display_expression();
        match self.evaluate_current() {
            Ok(value) => {
                self.answer = format_result(value);
                self.evaluated_formula = formula;
                self.just_evaluated = true;
            }
            Err(_) => {
                self.answer = "Error".to_string();
                self.just_evaluated = false;
            }
        }
    }

    fn refresh(&mut self) {
        if self.just_evaluated {
            return;
        }

        if self.pieces.is_empty() {
            self.answer = "0".to_string();
        }
    }

    fn current_value(&self) -> Option<f64> {
        if self.pieces.is_empty() && self.just_evaluated {
            return parse_answer(&self.answer);
        }
        self.evaluate_current()
            .ok()
            .or_else(|| parse_answer(&self.answer))
    }

    fn evaluate_current(&self) -> Result<f64, CalcError> {
        let expr = self.autoclosed_internal_expression();
        eval_expression(&expr, self.radians_mode)
    }

    fn display_expression(&self) -> String {
        self.pieces
            .iter()
            .map(|piece| piece.display.as_str())
            .collect()
    }

    fn internal_expression(&self) -> String {
        self.pieces
            .iter()
            .map(|piece| piece.internal.as_str())
            .collect()
    }

    fn autoclosed_internal_expression(&self) -> String {
        let mut expr = self.internal_expression();
        let mut balance = 0i32;
        for piece in &self.pieces {
            balance += piece.internal.matches('(').count() as i32;
            balance -= piece.internal.matches(')').count() as i32;
        }
        for _ in 0..balance.max(0) {
            expr.push(')');
        }
        expr
    }

    fn autoclosed_display_expression(&self) -> String {
        let mut expr = self.display_expression();
        let mut balance = 0i32;
        for piece in &self.pieces {
            balance += piece.display.matches('(').count() as i32;
            balance -= piece.display.matches(')').count() as i32;
        }
        for _ in 0..balance.max(0) {
            expr.push(')');
        }
        expr
    }

    fn prepare_for_value(&mut self) {
        if self.just_evaluated {
            self.clear();
        } else if self.ends_value() {
            self.push_piece("*", "×", PieceKind::Operator);
        }
    }

    fn prepare_for_operator(&mut self) {
        if self.just_evaluated {
            let value = parse_answer(&self.answer).unwrap_or(0.0);
            self.pieces = pieces_from_number(format_number_internal(value));
            self.just_evaluated = false;
            self.evaluated_formula.clear();
        }
    }

    fn push_digit(&mut self, digit: &str) {
        if self.just_evaluated {
            self.clear();
        }
        self.push_piece(digit, digit, PieceKind::Number);
    }

    fn push_decimal(&mut self) {
        if self.just_evaluated {
            self.clear();
        }
        if !self.current_number_has_decimal() {
            if !self.ends_number() {
                self.push_piece("0", "0", PieceKind::Number);
            }
            self.push_piece(".", ".", PieceKind::Decimal);
        }
    }

    fn push_exponent_marker(&mut self) {
        if self.just_evaluated {
            self.pieces = pieces_from_number(self.answer.replace('−', "-"));
            self.just_evaluated = false;
        }
        if !self.ends_number() {
            self.push_piece("1", "1", PieceKind::Number);
        }
        if !self.current_number_has_exponent() {
            self.push_piece("E", "E", PieceKind::Exponent);
        }
    }

    fn insert_number_literal(&mut self, number: String) {
        self.prepare_for_value();
        self.pieces.extend(pieces_from_number(number));
    }

    fn insert_constant(&mut self, internal: &str, display: &str) {
        self.prepare_for_value();
        self.push_piece(internal, display, PieceKind::Constant);
    }

    fn push_open_paren(&mut self) {
        self.prepare_for_value();
        self.push_piece("(", "(", PieceKind::OpenParen);
    }

    fn push_close_paren(&mut self) {
        if self.just_evaluated || self.pieces.is_empty() || self.last_is_operator_or_open() {
            return;
        }
        if self.open_paren_balance() > 0 {
            self.push_piece(")", ")", PieceKind::CloseParen);
        }
    }

    fn push_binary_operator(&mut self, internal: &str, display: &str) -> &mut Self {
        self.prepare_for_operator();

        if self.pieces.is_empty() {
            if internal == "-" {
                self.push_piece("-", "−", PieceKind::Operator);
            }
            return self;
        }

        if self.last_is_operator() {
            if let Some(last) = self.pieces.last_mut() {
                last.internal = internal.to_string();
                last.display = display.to_string();
            }
        } else if !self.last_is_function_start_or_open() {
            self.push_piece(internal, display, PieceKind::Operator);
        } else if internal == "-" {
            self.push_piece("-", "−", PieceKind::Operator);
        }
        self
    }

    fn push_power_literal(&mut self, number: &str) {
        if self.just_evaluated {
            self.pieces = pieces_from_number(self.answer.replace('−', "-"));
            self.just_evaluated = false;
        }
        if self.ends_value() {
            self.push_piece("^", "^", PieceKind::Operator);
            self.pieces.extend(pieces_from_number(number.to_string()));
        }
    }

    fn push_yroot_operator(&mut self) {
        self.prepare_for_operator();

        if let Some(start) = self.find_last_operand_start() {
            self.pieces
                .insert(start, Piece::new("", "root(", PieceKind::FunctionStart));
            self.push_piece("#", ",", PieceKind::Operator);
        }
    }

    fn push_logbase_operator(&mut self) {
        self.prepare_for_operator();

        if let Some(start) = self.find_last_operand_start() {
            self.pieces
                .insert(start, Piece::new("", "log(", PieceKind::FunctionStart));
            self.push_piece("@", ",", PieceKind::Operator);
        }
    }

    fn push_postfix(&mut self, internal: &str, display: &str) {
        if self.just_evaluated {
            self.pieces = pieces_from_number(self.answer.replace('−', "-"));
            self.just_evaluated = false;
        }
        if self.ends_value() {
            self.push_piece(internal, display, PieceKind::Postfix);
        }
    }

    fn apply_unary_function(&mut self, internal: &str, display: &str) {
        if self.just_evaluated {
            self.pieces = pieces_from_number(self.answer.replace('−', "-"));
            self.just_evaluated = false;
        }
        self.wrap_or_start(&format!("{internal}("), &format!("{display}("));
    }

    fn apply_power_prefix(&mut self, base_internal: &str, base_display: &str) {
        if self.just_evaluated {
            self.pieces = pieces_from_number(self.answer.replace('−', "-"));
            self.just_evaluated = false;
        }

        if let Some(start) = self.find_last_operand_start() {
            self.pieces.insert(
                start,
                Piece::new(
                    format!("{base_internal}^("),
                    format!("{base_display}("),
                    PieceKind::FunctionStart,
                ),
            );
            self.push_piece(")", ")", PieceKind::CloseParen);
        } else {
            self.prepare_for_value();
            self.push_piece(
                format!("{base_internal}^("),
                format!("{base_display}("),
                PieceKind::FunctionStart,
            );
        }
    }

    fn wrap_or_start(&mut self, prefix_internal: &str, prefix_display: &str) {
        if let Some(start) = self.find_last_operand_start() {
            self.pieces.insert(
                start,
                Piece::new(prefix_internal, prefix_display, PieceKind::FunctionStart),
            );
            self.push_piece(")", ")", PieceKind::CloseParen);
        } else {
            self.prepare_for_value();
            self.push_piece(prefix_internal, prefix_display, PieceKind::FunctionStart);
        }
    }

    fn toggle_sign(&mut self) {
        if self.just_evaluated {
            let value = parse_answer(&self.answer).unwrap_or(0.0);
            self.pieces = pieces_from_number(format_number_internal(-value));
            self.just_evaluated = false;
            self.evaluated_formula.clear();
            return;
        }

        if let Some(start) = self.find_last_operand_start() {
            if self.is_unary_minus_at(start) {
                self.pieces.remove(start);
            } else {
                self.pieces
                    .insert(start, Piece::new("-", "−", PieceKind::Operator));
            }
        } else {
            self.push_piece("-", "−", PieceKind::Operator);
        }
    }

    fn push_piece(
        &mut self,
        internal: impl Into<String>,
        display: impl Into<String>,
        kind: PieceKind,
    ) {
        self.pieces.push(Piece::new(internal, display, kind));
    }

    fn open_paren_balance(&self) -> i32 {
        let mut balance = 0i32;
        for piece in &self.pieces {
            balance += piece.internal.matches('(').count() as i32;
            balance -= piece.internal.matches(')').count() as i32;
        }
        balance
    }

    fn ends_value(&self) -> bool {
        matches!(
            self.pieces.last().map(|piece| piece.kind),
            Some(
                PieceKind::Number
                    | PieceKind::Decimal
                    | PieceKind::Exponent
                    | PieceKind::CloseParen
                    | PieceKind::Constant
                    | PieceKind::Postfix
            )
        )
    }

    fn ends_number(&self) -> bool {
        matches!(
            self.pieces.last().map(|piece| piece.kind),
            Some(PieceKind::Number | PieceKind::Decimal | PieceKind::Exponent)
        )
    }

    fn last_is_operator(&self) -> bool {
        matches!(
            self.pieces.last().map(|piece| piece.kind),
            Some(PieceKind::Operator)
        )
    }

    fn last_is_operator_or_open(&self) -> bool {
        self.last_is_operator() || self.last_is_function_start_or_open()
    }

    fn last_is_function_start_or_open(&self) -> bool {
        matches!(
            self.pieces.last().map(|piece| piece.kind),
            Some(PieceKind::OpenParen | PieceKind::FunctionStart)
        )
    }

    fn current_number_has_decimal(&self) -> bool {
        self.current_number_contains(".")
    }

    fn current_number_has_exponent(&self) -> bool {
        self.current_number_contains("E")
    }

    fn current_number_contains(&self, needle: &str) -> bool {
        for piece in self.pieces.iter().rev() {
            match piece.kind {
                PieceKind::Number | PieceKind::Decimal | PieceKind::Exponent => {
                    if piece.internal == needle {
                        return true;
                    }
                }
                _ => break,
            }
        }
        false
    }

    fn find_last_operand_start(&self) -> Option<usize> {
        let len = self.pieces.len();
        if len == 0 || self.last_is_operator_or_open() {
            return None;
        }

        let mut end = len;
        while end > 0 && self.pieces[end - 1].kind == PieceKind::Postfix {
            end -= 1;
        }
        if end == 0 {
            return None;
        }

        let start = match self.pieces[end - 1].kind {
            PieceKind::CloseParen => self.find_matching_open(end - 1)?,
            PieceKind::Constant => end - 1,
            PieceKind::Number | PieceKind::Decimal | PieceKind::Exponent => {
                let mut start = end - 1;
                while start > 0
                    && matches!(
                        self.pieces[start - 1].kind,
                        PieceKind::Number | PieceKind::Decimal | PieceKind::Exponent
                    )
                {
                    start -= 1;
                }
                start
            }
            _ => return None,
        };

        if start > 0 && self.is_unary_minus_at(start - 1) {
            Some(start - 1)
        } else {
            Some(start)
        }
    }

    fn find_matching_open(&self, close_index: usize) -> Option<usize> {
        let mut depth = 0i32;
        for index in (0..=close_index).rev() {
            match self.pieces[index].kind {
                PieceKind::CloseParen => depth += 1,
                PieceKind::OpenParen | PieceKind::FunctionStart => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn is_unary_minus_at(&self, index: usize) -> bool {
        self.pieces
            .get(index)
            .is_some_and(|piece| piece.kind == PieceKind::Operator && piece.internal == "-")
            && (index == 0
                || matches!(
                    self.pieces[index - 1].kind,
                    PieceKind::Operator | PieceKind::OpenParen | PieceKind::FunctionStart
                ))
    }

    fn next_random(&mut self) -> f64 {
        self.random_seed = self
            .random_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mantissa = self.random_seed >> 11;
        mantissa as f64 / ((1u64 << 53) as f64)
    }
}

fn pieces_from_number(number: String) -> Vec<Piece> {
    number
        .chars()
        .map(|ch| match ch {
            '-' => Piece::new("-", "−", PieceKind::Operator),
            '.' => Piece::new(".", ".", PieceKind::Decimal),
            'E' | 'e' => Piece::new("E", "E", PieceKind::Exponent),
            _ => Piece::new(ch.to_string(), ch.to_string(), PieceKind::Number),
        })
        .collect()
}

fn parse_answer(answer: &str) -> Option<f64> {
    answer.replace('−', "-").parse().ok()
}

fn format_number_internal(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let value = if value.abs() < 1e-12 { 0.0 } else { value };
    if value.fract().abs() < 1e-12 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        let mut text = format!("{value:.12}");
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        if text.len() > 18 {
            text = format!("{value:.10e}").replace('e', "E");
        }
        text
    }
}

fn format_result(value: f64) -> String {
    if !value.is_finite() {
        return "Error".to_string();
    }
    format_number_internal(value).replace('-', "−")
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Root,
    LogBase,
    LParen,
    RParen,
    Factorial,
    Percent,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalcError {
    InvalidToken,
    UnexpectedToken,
    Domain,
    DivideByZero,
}

fn eval_expression(input: &str, radians_mode: bool) -> Result<f64, CalcError> {
    let tokens = lex(input)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        radians_mode,
    };
    let value = parser.parse_expression()?;
    if parser.current() != &Token::End {
        return Err(CalcError::UnexpectedToken);
    }
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CalcError::Domain)
    }
}

fn lex(input: &str) -> Result<Vec<Token>, CalcError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' | b'.' => {
                let start = index;
                index += 1;
                while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.')
                {
                    index += 1;
                }
                if index < bytes.len() && (bytes[index] == b'E' || bytes[index] == b'e') {
                    index += 1;
                    if index < bytes.len() && (bytes[index] == b'+' || bytes[index] == b'-') {
                        index += 1;
                    }
                    while index < bytes.len() && bytes[index].is_ascii_digit() {
                        index += 1;
                    }
                }
                let number = input[start..index]
                    .parse::<f64>()
                    .map_err(|_| CalcError::InvalidToken)?;
                tokens.push(Token::Number(number));
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                let start = index;
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                    index += 1;
                }
                tokens.push(Token::Ident(input[start..index].to_string()));
            }
            b'+' => {
                tokens.push(Token::Plus);
                index += 1;
            }
            b'-' => {
                tokens.push(Token::Minus);
                index += 1;
            }
            b'*' => {
                tokens.push(Token::Star);
                index += 1;
            }
            b'/' => {
                tokens.push(Token::Slash);
                index += 1;
            }
            b'^' => {
                tokens.push(Token::Caret);
                index += 1;
            }
            b'#' => {
                tokens.push(Token::Root);
                index += 1;
            }
            b'@' => {
                tokens.push(Token::LogBase);
                index += 1;
            }
            b'(' => {
                tokens.push(Token::LParen);
                index += 1;
            }
            b')' => {
                tokens.push(Token::RParen);
                index += 1;
            }
            b'!' => {
                tokens.push(Token::Factorial);
                index += 1;
            }
            b'%' => {
                tokens.push(Token::Percent);
                index += 1;
            }
            b' ' | b'\t' | b'\n' => index += 1,
            _ => return Err(CalcError::InvalidToken),
        }
    }

    tokens.push(Token::End);
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    radians_mode: bool,
}

impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) {
        self.pos = (self.pos + 1).min(self.tokens.len() - 1);
    }

    fn parse_expression(&mut self) -> Result<f64, CalcError> {
        let mut value = self.parse_term()?;
        loop {
            match self.current() {
                Token::Plus => {
                    self.bump();
                    value += self.parse_term()?;
                }
                Token::Minus => {
                    self.bump();
                    value -= self.parse_term()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, CalcError> {
        let mut value = self.parse_power()?;
        loop {
            match self.current() {
                Token::Star => {
                    self.bump();
                    value *= self.parse_power()?;
                }
                Token::Slash => {
                    self.bump();
                    let rhs = self.parse_power()?;
                    if rhs.abs() < f64::EPSILON {
                        return Err(CalcError::DivideByZero);
                    }
                    value /= rhs;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_power(&mut self) -> Result<f64, CalcError> {
        let value = self.parse_unary()?;
        match self.current() {
            Token::Caret => {
                self.bump();
                checked(value.powf(self.parse_power()?))
            }
            Token::Root => {
                self.bump();
                let radicand = self.parse_power()?;
                if value.abs() < f64::EPSILON {
                    return Err(CalcError::DivideByZero);
                }
                checked(radicand.powf(1.0 / value))
            }
            Token::LogBase => {
                self.bump();
                let argument = self.parse_power()?;
                if value <= 0.0 || (value - 1.0).abs() < f64::EPSILON || argument <= 0.0 {
                    return Err(CalcError::Domain);
                }
                checked(argument.ln() / value.ln())
            }
            _ => Ok(value),
        }
    }

    fn parse_unary(&mut self) -> Result<f64, CalcError> {
        match self.current() {
            Token::Plus => {
                self.bump();
                self.parse_unary()
            }
            Token::Minus => {
                self.bump();
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<f64, CalcError> {
        let mut value = self.parse_primary()?;
        loop {
            match self.current() {
                Token::Percent => {
                    self.bump();
                    value /= 100.0;
                }
                Token::Factorial => {
                    self.bump();
                    value = factorial(value)?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_primary(&mut self) -> Result<f64, CalcError> {
        match self.current().clone() {
            Token::Number(value) => {
                self.bump();
                Ok(value)
            }
            Token::Ident(name) => {
                self.bump();
                match name.as_str() {
                    "pi" => Ok(PI),
                    "econst" => Ok(E),
                    _ => {
                        if self.current() != &Token::LParen {
                            return Err(CalcError::UnexpectedToken);
                        }
                        self.bump();
                        let arg = self.parse_expression()?;
                        if self.current() != &Token::RParen {
                            return Err(CalcError::UnexpectedToken);
                        }
                        self.bump();
                        self.apply_function(&name, arg)
                    }
                }
            }
            Token::LParen => {
                self.bump();
                let value = self.parse_expression()?;
                if self.current() != &Token::RParen {
                    return Err(CalcError::UnexpectedToken);
                }
                self.bump();
                Ok(value)
            }
            _ => Err(CalcError::UnexpectedToken),
        }
    }

    fn apply_function(&self, name: &str, arg: f64) -> Result<f64, CalcError> {
        let value = match name {
            "sqrt" => {
                if arg < 0.0 {
                    return Err(CalcError::Domain);
                }
                arg.sqrt()
            }
            "cbrt" => arg.cbrt(),
            "ln" => {
                if arg <= 0.0 {
                    return Err(CalcError::Domain);
                }
                arg.ln()
            }
            "log10" => {
                if arg <= 0.0 {
                    return Err(CalcError::Domain);
                }
                arg.log10()
            }
            "log2" => {
                if arg <= 0.0 {
                    return Err(CalcError::Domain);
                }
                arg.log2()
            }
            "sin" => self.angle_in(arg).sin(),
            "cos" => self.angle_in(arg).cos(),
            "tan" => self.angle_in(arg).tan(),
            "asin" => self.angle_out(arg.asin()),
            "acos" => self.angle_out(arg.acos()),
            "atan" => self.angle_out(arg.atan()),
            "sinh" => arg.sinh(),
            "cosh" => arg.cosh(),
            "tanh" => arg.tanh(),
            "asinh" => arg.asinh(),
            "acosh" => {
                if arg < 1.0 {
                    return Err(CalcError::Domain);
                }
                arg.acosh()
            }
            "atanh" => {
                if arg <= -1.0 || arg >= 1.0 {
                    return Err(CalcError::Domain);
                }
                arg.atanh()
            }
            _ => return Err(CalcError::UnexpectedToken),
        };
        checked(value)
    }

    fn angle_in(&self, value: f64) -> f64 {
        if self.radians_mode {
            value
        } else {
            value.to_radians()
        }
    }

    fn angle_out(&self, value: f64) -> f64 {
        if self.radians_mode {
            value
        } else {
            value.to_degrees()
        }
    }
}

fn checked(value: f64) -> Result<f64, CalcError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CalcError::Domain)
    }
}

fn factorial(value: f64) -> Result<f64, CalcError> {
    if value < 0.0 || value > 170.0 || (value - value.round()).abs() > 1e-10 {
        return Err(CalcError::Domain);
    }
    let mut result = 1.0;
    for n in 2..=(value.round() as u64) {
        result *= n as f64;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(input: &str) -> f64 {
        eval_expression(input, false).unwrap()
    }

    #[test]
    fn respects_operator_precedence() {
        assert_eq!(eval("2+3*4"), 14.0);
        assert_eq!(eval("(2+3)*4"), 20.0);
    }

    #[test]
    fn supports_scientific_functions() {
        assert!((eval("sin(90)") - 1.0).abs() < 1e-10);
        assert_eq!(eval("sqrt(9)"), 3.0);
        assert_eq!(eval("2^3"), 8.0);
        assert_eq!(eval("2#9"), 3.0);
        assert_eq!(eval("2@8"), 3.0);
        assert_eq!(eval("5!"), 120.0);
    }

    #[test]
    fn builds_formula_and_answer() {
        let mut calc = Calculator::new();
        for input in ["7", "multiply", "7"] {
            calc.press(input);
        }
        assert_eq!(calc.formula_text(), "7×7");
        assert_eq!(calc.answer_text(), "0");
        assert!(!calc.result_mode());
        calc.press("equals");
        assert_eq!(calc.formula_text(), "7×7");
        assert_eq!(calc.answer_text(), "49");
        assert!(calc.result_mode());
    }

    #[test]
    fn does_not_preview_result_before_equals() {
        let mut calc = Calculator::new();
        for input in ["9", "multiply", "9", "plus", "6"] {
            calc.press(input);
        }
        assert_eq!(calc.formula_text(), "9×9+6");
        assert_eq!(calc.answer_text(), "0");
        assert!(!calc.result_mode());
        calc.press("equals");
        assert_eq!(calc.formula_text(), "9×9+6");
        assert_eq!(calc.answer_text(), "87");
        assert!(calc.result_mode());
    }

    #[test]
    fn applies_square_root_before_or_after_operand() {
        let mut calc = Calculator::new();
        calc.press("9");
        calc.press("sqrt");
        assert_eq!(calc.formula_text(), "sqrt(9)");
        assert_eq!(calc.answer_text(), "0");
        calc.press("equals");
        assert_eq!(calc.answer_text(), "3");
        assert!(calc.result_mode());

        let mut calc = Calculator::new();
        calc.press("sqrt");
        calc.press("9");
        assert_eq!(calc.formula_text(), "sqrt(9");
        assert_eq!(calc.answer_text(), "0");
        calc.press("equals");
        assert_eq!(calc.formula_text(), "sqrt(9)");
        assert_eq!(calc.answer_text(), "3");
        assert!(calc.result_mode());
    }

    #[test]
    fn displays_root_functions_without_special_glyphs() {
        let mut calc = Calculator::new();
        calc.press("2");
        calc.press("7");
        calc.press("cbrt");
        assert_eq!(calc.formula_text(), "cbrt(27)");
        assert_eq!(calc.answer_text(), "0");
        calc.press("equals");
        assert_eq!(calc.formula_text(), "cbrt(27)");
        assert_eq!(calc.answer_text(), "3");

        let mut calc = Calculator::new();
        calc.press("3");
        calc.press("yroot");
        calc.press("2");
        calc.press("7");
        assert_eq!(calc.formula_text(), "root(3,27");
        assert_eq!(calc.answer_text(), "0");
        calc.press("equals");
        assert_eq!(calc.formula_text(), "root(3,27)");
        assert_eq!(calc.answer_text(), "3");
    }

    #[test]
    fn displays_log_base_without_subscript_glyphs() {
        let mut calc = Calculator::new();
        calc.press("2");
        calc.press("logy");
        calc.press("8");
        assert_eq!(calc.formula_text(), "log(2,8");
        assert_eq!(calc.answer_text(), "0");
        calc.press("equals");
        assert_eq!(calc.formula_text(), "log(2,8)");
        assert_eq!(calc.answer_text(), "3");
    }
}
