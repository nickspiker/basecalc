// Basecalc: Your Towel in the Mathematical Cosmos
// Copyright (C) 2024 Nick Spiker
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this program.  If not, see <https://www.gnu.org/licenses/>.


//! basecalc's engine: the tokenizer, the evaluator over rug's Complex (GMP, MPFR and MPC), the commands that change the state, and the formatter that writes a result in any base 2 to 36.
//! No terminal, no files: the terminal front is the `basecalc` binary (src/main.rs), and a front of any other kind (a ferros kist's keypad) drives the same functions.
//! Text comes back as [`Span`]s, each tagged with its [`Role`] (an integer part, a fraction, the exponent, a bracket), so each front colours them its own way.

use az::Cast;
use rug::ops::*;
use rug::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// What a piece of formatted text is, so a front can colour it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Plain,
    LoneInteger,
    LoneFraction,
    RealInteger,
    RealFraction,
    ImaginaryInteger,
    ImaginaryFraction,
    Exponent,
    Decimal,
    Sign,
    Tilde,
    Carat,
    Error,
    Brackets,
    Comma,
    Colon,
    Nan,
    Message,
}

/// A run of text and its role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub role: Role,
    pub text: String,
}

pub fn span(role: Role, text: impl Into<String>) -> Span {
    Span { role, text: text.into() }
}

/// The spans' text run together, the roles dropped.
pub fn spans_text(spans: &[Span]) -> String {
    let mut s = String::new();
    for p in spans {
        s.push_str(&p.text);
    }
    s
}

pub struct EvalResult {
    pub value: Complex,
    pub assignment: Option<usize>, // Index of assigned variable, if this was an assignment
}
#[derive(Clone)]
pub struct Variable {
    pub name: String,
    pub value: Complex,
}
#[derive(Clone)]
pub struct BasecalcState {
    pub base: u8,
    pub digits: usize,
    pub precision: u32,
    pub padding: u32,
    pub radians: bool,
    pub current_entry: String,
    pub history_index: usize,
    pub history: Vec<String>,
    pub debug: bool,
    pub rand_state: rand::RandState<'static>,
    pub prev_result: Complex,
    pub colours: RGBValues,
    pub variables: Vec<Variable>,
}

impl BasecalcState {
    pub fn new() -> Self {
        let base = 10;
        let digits = 12;
        let precision = 0;
        let mut state = BasecalcState {
            base,
            digits,
            precision,
            padding: 32,
            radians: true,
            current_entry: String::new(),
            history_index: 0,
            history: Vec::new(),
            debug: false,
            rand_state: rand::RandState::new(),
            prev_result: Complex::with_val(1, 0),
            colours: RGBValues {
                lone_integer: (0x94, 0xc9, 0x9b),
                lone_fraction: (0x6a, 0xce, 0xb0),
                real_integer: (0x81, 0xc6, 0xdc),
                real_fraction: (0xa5, 0xbe, 0xe7),
                imaginary_integer: (0xe5, 0xae, 0xa0),
                imaginary_fraction: (0xf9, 0xa0, 0xc8),
                exponent: (0x9C, 0x27, 0xB0),
                decimal: (0xFF, 0xff, 0xff),
                sign: (0xF4, 0x43, 0x36),
                tilde: (0x78, 0x90, 0xCC),
                carat: (0xFF, 0xC1, 0x07),
                error: (0xE5, 0x39, 0x35),
                brackets: (0x8B, 0xC3, 0x4A),
                comma: (0xBD, 0xBD, 0xBD),
                colon: (0x78, 0x90, 0x9C),
                nan: (0xc0, 0x0D, 0xfB),
                message: (0x9E, 0x35, 0xe1),
            },
            variables: Vec::new(),
        };
        state.set_precision();
        state.prev_result = Complex::with_val(state.precision, 0);
        state
    }
    pub fn set_precision(&mut self) {
        self.precision =
            (self.digits as f64 * (self.base as f64).log2()).ceil() as u32 + self.padding;
    }
}
pub static OPERATORS: [(&str, char, u8, &str); 30] = [
    // Basic arithmetic
    ("+", '+', 2, "addition"),
    ("-", '-', 2, "subtraction"),
    ("*", '*', 2, "multiplication"),
    ("/", '/', 2, "division"),
    ("^", '^', 2, "exponentiation"),
    ("%", '%', 2, "modulus"),
    ("$", '$', 2, "logarithm (number$base = log base of number)"),
    // Parentheses
    ("(", '(', 1, "left parenthesis"),
    (")", ')', 1, "right parenthesis"),
    // Common functions
    ("#sqrt", 'q', 1, "square root"),
    ("#abs", 'a', 1, "absolute value"),
    ("#ln", 'l', 1, "natural logarithm"),
    ("#log", 'L', 1, "base logarithm"),
    // Trigonometric functions
    ("#sin", 's', 1, "sine"),
    ("#cos", 'o', 1, "cosine"),
    ("#tan", 't', 1, "tangent"),
    ("#asin", 'S', 1, "inverse sine"),
    ("#acos", 'O', 1, "inverse cosine"),
    ("#atan", 'T', 1, "inverse tangent"),
    // Rounding and parts
    ("#ceil", 'c', 1, "gaussian ceiling"),
    ("#floor", 'f', 1, "gaussian floor"),
    ("#round", 'r', 1, "gaussian rounding"),
    ("#int", 'I', 1, "integer part"),
    ("#frac", 'F', 1, "fractional part"),
    // Complex number operations
    ("#re", 'e', 1, "real"),
    ("#im", 'i', 1, "imaginary"),
    ("#angle", 'A', 1, "complex angle"),
    // Miscellaneous
    ("#sign", 'g', 1, "sign"),
    ("#erf", 'x', 1, "error function"),
    ("=", '=', 2, "assignment"),
    // ("#gamma", '!', 1, "gamma function"),
    // ("#max", 'M', 2, "maximum"),
    // ("#min", 'm', 2, "minimum"),
];
pub static CONSTANTS: [(&str, char, &str); 8] = [
    ("@pi", 'p', "Pi"),
    ("@phi", 'P', "Golden ratio"),
    ("@e", 'E', "Euler's number"),
    ("@gamma", 'G', "Euler-Mascheroni constant"),
    ("@catalan", 'C', "Catalan's constant"),
    ("@rand", 'r', "Random number between 0 and 1"),
    ("@grand", 'g', "Gaussian random number"),
    ("&", '&', "Previous result"),
];
#[derive(Clone)]
pub struct RGBValues {
    pub lone_integer: (u8, u8, u8),
    pub lone_fraction: (u8, u8, u8),
    pub real_integer: (u8, u8, u8),
    pub real_fraction: (u8, u8, u8),
    pub imaginary_integer: (u8, u8, u8),
    pub imaginary_fraction: (u8, u8, u8),
    pub exponent: (u8, u8, u8),
    pub decimal: (u8, u8, u8),
    pub sign: (u8, u8, u8),
    pub tilde: (u8, u8, u8),
    pub carat: (u8, u8, u8),
    pub error: (u8, u8, u8),
    pub brackets: (u8, u8, u8),
    pub comma: (u8, u8, u8),
    pub colon: (u8, u8, u8),
    pub nan: (u8, u8, u8),
    pub message: (u8, u8, u8),
}
pub static DEBUG: AtomicBool = AtomicBool::new(false);
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Precedence {
    Addition,
    Multiplication,
    Exponentiation,
    Unary,
    Parenthesis,
    Assignment,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Token {
    pub operator: char,
    pub operands: u8,
    pub real_integer: Vec<u8>,
    pub real_fraction: Vec<u8>,
    pub imaginary_integer: Vec<u8>,
    pub imaginary_fraction: Vec<u8>,
    pub sign: (bool, bool),
    pub var_index: Option<usize>,
}
use std::fmt;
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn number_vector_to_string(vec: &[u8]) -> String {
            let mut s = String::new();
            for i in 0..vec.len() {
                let c = vec[i];
                if c > 9 {
                    s.push((c - 10 + b'A') as char);
                } else {
                    s.push((c + b'0') as char);
                }
            }
            s
        }
        if self.operator as u8 > 1 {
            write!(f, "{}:", self.operator)?;
        } else if self.operator as u8 == 1 {
            write!(f, "№:")?;
        }

        write!(f, "{}[", self.operands)?;

        if self.sign.0 {
            write!(f, "-")?;
        } else {
            write!(f, "+")?;
        }
        write!(f, "{}", number_vector_to_string(&self.real_integer))?;
        write!(f, ".{} , ", number_vector_to_string(&self.real_fraction))?;

        if self.sign.1 {
            write!(f, "-")?;
        } else {
            write!(f, "+")?;
        }
        write!(f, "{}", number_vector_to_string(&self.imaginary_integer))?;
        write!(f, ".{}", number_vector_to_string(&self.imaginary_fraction))?;

        write!(f, "]")
    }
}
impl Token {
    pub fn new() -> Token {
        Token {
            operator: 0 as char,
            operands: 0,
            real_integer: Vec::new(),
            real_fraction: Vec::new(),
            imaginary_integer: Vec::new(),
            imaginary_fraction: Vec::new(),
            sign: (false, false),
            var_index: None,
        }
    }
}
pub trait Modulus {
    fn modulus(&self, modulor: Complex) -> Complex;
}
impl Modulus for Complex {
    fn modulus(&self, modulor: Complex) -> Complex {
        let real = if modulor.real().is_zero() {
            Float::with_val(self.real().prec(), 0) // Avoid division by zero
        } else {
            self.real().clone()
                - (modulor.real().clone() * (self.real().clone() / modulor.real().clone()).floor())
        };
        let imaginary = if modulor.imag().is_zero() {
            Float::with_val(self.imag().prec(), 0) // Avoid division by zero
        } else {
            self.imag().clone()
                - (modulor.imag().clone() * (self.imag().clone() / modulor.imag().clone()).floor())
        };
        Complex::with_val(self.prec(), (real, imaginary))
    }
}
/// Tokenizes the input string into a vector of Tokens
///
/// # Arguments
/// * `input_str` - The input string to tokenize
/// * `base` - The current number base
/// * `precision` - The current precision for calculations
/// * `digits` - The number of digits to display in results
/// * `radians` - Whether to use radians for trigonometric functions
/// * `colours` - The colour scheme for output formatting
///
/// # Returns
/// * `Ok(Vec<Token>)` - A vector of tokens if successful
/// * `Err((String, usize))` - An error message and the position of the error
/// Tokenizes the input string into a vector of Tokens
///
/// # Arguments
/// * `input_str` - The input string to tokenize
/// * `base` - The current number base
/// * `precision` - The current precision for calculations
/// * `digits` - The number of digits to display in results
/// * `radians` - Whether to use radians for trigonometric functions
/// * `colours` - The colour scheme for output formatting
///
/// # Returns
/// * `Ok(Vec<Token>)` - A vector of tokens if successful
/// * `Err((String, usize))` - An error message and the position of the error
pub fn tokenize(input_str: &str, state: &mut BasecalcState) -> Result<Vec<Token>, Halt> {
    debug_println(&format!("\nTokenizing: {}", input_str));
    debug_println(&format!(
        "Initial state: base={}, precision={}, digits={}, radians={}",
        state.base, state.precision, state.digits, state.radians
    ));

    let input = input_str.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut paren_count = 0;
    let mut start = true;
    let mut expect_number = true;
    let mut follows_number = false;

    while index < input.len() {
        debug_println(&format!(
            "Processing character at index {}: '{}'",
            index, input[index] as char
        ));

        if input[index] == b' ' || input[index] == b'_' || input[index] == b'\t' {
            debug_println(&format!("Skipping whitespace"));
            index += 1;
            continue;
        }
        if start && input[index] == b':' {
            debug_println(&format!("Command detected, parsing command"));
            match parse_command(input, index + 1, state) {
                CommandResult::Success(msg) => return Err(Halt::Said(msg)),
                CommandResult::Error(msg, pos) => return Err(Halt::Wrong(msg, pos)),
                CommandResult::Silent => return Err(Halt::Said(String::new())),
                CommandResult::Help => return Err(Halt::Help),
                CommandResult::Tested(outcomes) => return Err(Halt::Tested(outcomes)),
                CommandResult::Dms(spans) => return Err(Halt::Dms(spans)),
            }
        }
        if input[index] == b'(' {
            if !start && follows_number {
                debug_println(&format!(
                    "Error: Expected operator, found opening parenthesis"
                ));
                return Err(Halt::Wrong(format!("Expected operator!"), index));
            }
            debug_println(&format!("Adding opening parenthesis token"));
            tokens.push(Token {
                operator: '(',
                operands: 1,
                ..Token::new()
            });
            paren_count += 1;
            index += 1;
            continue;
        }
        if input[index] == b')' {
            if paren_count == 0 {
                debug_println(&format!("Error: Mismatched parentheses"));
                return Err(Halt::Wrong(format!("Mismatched parentheses!"), index));
            }
            if !follows_number {
                debug_println(&format!(
                    "Error: Expected number before closing parenthesis"
                ));
                return Err(Halt::Wrong(format!("Expected number!"), index));
            }
            debug_println(&format!("Adding closing parenthesis token"));
            tokens.push(Token {
                operator: ')',
                operands: 1,
                ..Token::new()
            });
            paren_count -= 1;
            index += 1;
            continue;
        }
        if expect_number {
            debug_println(&format!("Expecting a number or constant"));
            match parse_constant(input, index, state) {
                Ok((token, new_index)) => {
                    debug_println(&format!("Parsed constant: {}", token));
                    tokens.push(token);
                    index = new_index;
                    start = false;
                    expect_number = false;
                    follows_number = true;
                    continue;
                }
                Err((_msg, _pos)) => {
                    debug_println(&format!("Not a constant, trying to parse as number"));
                }
            }
            match parse_number(input, state.base, index) {
                Ok((token, new_index)) => {
                    debug_println(&format!("Parsed number: {}", token));
                    tokens.push(token);
                    index = new_index;
                    start = false;
                    expect_number = false;
                    follows_number = true;
                    continue;
                }
                Err((msg, pos)) => {
                    debug_println(&format!(
                        "Failed to parse as number, attempting to parse as operator"
                    ));
                    let (mut token, new_index) = parse_operator(input, index);
                    if token.operator == '\0' || token.operands == 2 {
                        if token.operator == '-' {
                            token.operator = 'n';
                            token.operands = 1;
                            debug_println(&format!("Parsed unary negation operator: {}", token));
                            tokens.push(token);
                            index = new_index;
                            continue;
                        } else {
                            debug_println(&format!("Error: Invalid token"));
                            return Err(Halt::Wrong(msg, pos));
                        }
                    }
                    debug_println(&format!("Parsed unary operator: {}", token));
                    tokens.push(token);
                    index = new_index;
                    start = false;
                    expect_number = true;
                    continue;
                }
            }
        }
        let (token, new_index) = parse_operator(input, index);
        if token.operator == '\0' {
            debug_println(&format!("Error: Invalid operator"));
            return Err(Halt::Wrong(format!("Invalid operator!"), new_index));
        }
        if token.operands == 1 && follows_number {
            debug_println(&format!("Error: Expected binary operator, found unary"));
            return Err(Halt::Wrong(format!("Expected operator!"), index));
        }
        debug_println(&format!("Parsed operator: {}", token));
        tokens.push(token);
        index = new_index;
        expect_number = true;
        follows_number = false;
    }

    if paren_count != 0 {
        debug_println(&format!("Error: Mismatched parentheses at end of input"));
        return Err(Halt::Wrong(format!("Mismatched parentheses!"), input.len()));
    }

    if tokens.is_empty() {
        debug_println(&format!("Error: Empty expression"));
        return Err(Halt::Wrong(format!("Empty expression"), 0));
    }

    let last_token = tokens.last().unwrap();
    if last_token.operands > 0 && last_token.operator != ')' {
        debug_println(&format!("Error: Incomplete expression at end of input"));
        return Err(Halt::Wrong(format!("Incomplete expression!"), input.len()));
    }

    debug_println(&format!("Tokenization completed successfully"));
    for (i, token) in tokens.iter().enumerate() {
        debug_println(&format!("Token {}: {}", i, token));
    }

    Ok(tokens)
}
/// Evaluates a vector of tokens and returns the result
///
/// # Arguments
/// * `tokens` - The vector of tokens to evaluate
/// * `base` - The current number base
/// * `precision` - The precision for calculations
/// * `rand_state` - The random state for random number generation
/// * `radians` - Whether to use radians for trigonometric functions
///
/// # Returns
/// * `Ok(Complex)` - The result of the evaluation as a complex number
/// * `Err(String)` - An error message if evaluation fails
pub fn evaluate_tokens(tokens: &[Token], state: &mut BasecalcState) -> Result<EvalResult, String> {
    debug_println("\nEvaluating tokens:");

    // Check for variable assignment pattern (var = expr)
    if tokens.len() >= 2 && tokens[0].operator == 'v' && tokens[1].operator == '=' {
        // Get variable name and index
        let var_index = tokens[0].var_index.ok_or("Invalid variable reference")?;

        // Evaluate the right-hand side expression
        let mut output_queue: Vec<Complex> = Vec::new();
        let mut operator_stack: Vec<char> = Vec::new();

        // Process tokens after the '=' sign
        for token in &tokens[2..] {
            match token.operands {
                0 => {
                    let mut value = token2num(token, state);
                    while let Some(&op) = operator_stack.last() {
                        if get_precedence(op) == Precedence::Unary {
                            let operator = operator_stack.pop().unwrap();
                            value = apply_unary_operator(operator, value, state)?;
                        } else {
                            break;
                        }
                    }
                    output_queue.push(value);
                }
                1 => {
                    if token.operator == '(' {
                        operator_stack.push('(');
                    } else if token.operator == ')' {
                        while let Some(&op) = operator_stack.last() {
                            if op == '(' {
                                operator_stack.pop();
                                break;
                            }
                            apply_operator(&mut output_queue, operator_stack.pop().unwrap(), state)?;
                        }
                    } else {
                        operator_stack.push(token.operator);
                    }
                }
                2 => {
                    while let Some(&op) = operator_stack.last() {
                        if op == '(' || get_precedence(token.operator) > get_precedence(op) {
                            break;
                        }
                        apply_operator(&mut output_queue, operator_stack.pop().unwrap(), state)?;
                    }
                    operator_stack.push(token.operator);
                }
                _ => return Err(format!("Invalid token: {}", token)),
            }
        }

        while let Some(op) = operator_stack.pop() {
            if op == '(' {
                return Err("Mismatched parentheses".to_string());
            }
            apply_operator(&mut output_queue, op, state)?;
        }

        if output_queue.len() != 1 {
            return Err("Invalid expression".to_string());
        }

        let result = output_queue.pop().unwrap();
        state.variables[var_index].value = result.clone();
        
        Ok(EvalResult {
            value: result,
            assignment: Some(var_index)
        })

    } else {
        // Regular expression evaluation (unchanged)
        let mut output_queue: Vec<Complex> = Vec::new();
        let mut operator_stack: Vec<char> = Vec::new();

        for token in tokens {
            debug_println(&format!("Processing token: {}", token));
            match token.operands {
                0 => {
                    let mut value = token2num(token, state);
                    debug_println(&format!("Processing number: {}", value));

                    while let Some(&op) = operator_stack.last() {
                        if get_precedence(op) == Precedence::Unary {
                            debug_println(&format!("Applying stacked unary operator: {}", op));
                            let operator = operator_stack.pop().unwrap();
                            value = apply_unary_operator(operator, value, state)?;
                        } else {
                            break;
                        }
                    }

                    debug_println(&format!("Pushed processed number to output queue: {}", value));
                    output_queue.push(value);
                }
                1 => {
                    debug_println(&format!("Processing unary operator: {}", token.operator));
                    if token.operator == '(' {
                        operator_stack.push('(');
                        debug_println("Pushed opening parenthesis to stack");
                    } else if token.operator == ')' {
                        while let Some(&op) = operator_stack.last() {
                            if op == '(' {
                                operator_stack.pop();
                                break;
                            }
                            apply_operator(&mut output_queue, operator_stack.pop().unwrap(), state)?;
                        }
                        if let Some(&op) = operator_stack.last() {
                            if get_precedence(op) == Precedence::Unary {
                                apply_operator(&mut output_queue, operator_stack.pop().unwrap(), state)?;
                            }
                        }
                    } else {
                        debug_println(&format!("Pushed unary operator to stack: {}", token.operator));
                        operator_stack.push(token.operator);
                    }
                }
                2 => {
                    while let Some(&op) = operator_stack.last() {
                        if op == '(' || get_precedence(token.operator) > get_precedence(op) {
                            break;
                        }
                        apply_operator(&mut output_queue, operator_stack.pop().unwrap(), state)?;
                    }
                    operator_stack.push(token.operator);
                    debug_println(&format!("Pushed binary operator to stack: {}", token.operator));
                }
                _ => return Err(format!("Invalid token: {}", token)),
            }
            debug_println(&format!("Output queue: {:?}", output_queue));
            debug_println(&format!("Operator stack: {:?}", operator_stack));
        }

        while let Some(op) = operator_stack.pop() {
            if op == '(' {
                return Err("Mismatched parentheses".to_string());
            }
            debug_println(&format!("Applying remaining operator: {}", op));
            apply_operator(&mut output_queue, op, state)?;
        }

        if output_queue.len() != 1 {
            return Err("Invalid expression".to_string());
        }

        Ok(EvalResult {
            value: output_queue.pop().unwrap(),
            assignment: None
        })
    }
}
pub fn apply_operator(
    output_queue: &mut Vec<Complex>,
    op: char,
    state: &mut BasecalcState,
) -> Result<(), String> {
    debug_println(&format!("Applying operator: {}", op));
    match op {
        '+' | '-' | '*' | '/' | '^' | '%' | '$' => apply_binary_operator(output_queue, op)?,
        'n' | 'a' | 'O' | 'o' | 'S' | 'T' | 'c' | 'f' | 'F' | 'i' | 'I' | 'l' | 'L' | 'e' | 'r'
        | 'g' | 's' | 'q' | 't' | 'A' | 'x' => {
            if let Some(value) = output_queue.pop() {
                let result = apply_unary_operator(op, value, state)?;
                output_queue.push(result);
            } else {
                return Err(format!("Not enough operands for {}", op));
            }
        }
        _ => return Err(format!("Unknown operator: {}", op)),
    }
    Ok(())
}
pub fn get_precedence(op: char) -> Precedence {
    match op {
        '+' | '-' => Precedence::Addition,
        '*' | '/' | '%' => Precedence::Multiplication,
        '^' | '$' => Precedence::Exponentiation,
        'n' | 'a' | 'O' | 'o' | 'S' | 'T' | 'c' | 'f' | 'F' | 'i' | 'I' | 'l' | 'L' | 'e' | 'r'
        | 'g' | 's' | 'q' | 't' | 'A' => Precedence::Unary,
        '(' | ')' => Precedence::Parenthesis,
        '=' => Precedence::Assignment,
        _ => Precedence::Addition, // Default to lowest precedence for unknown operators
    }
}
pub fn apply_unary_operator(
    op: char,
    value: Complex,
    state: &BasecalcState,
) -> Result<Complex, String> {
    debug_println(&format!(
        "Applying unary operator: {} to value: {}",
        op, value
    ));
    let result = match op {
        'n' => -value,
        'a' => value.abs(),
        'S' => {
            let rad_result = value.asin();
            if state.radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(state.precision, rug::float::Constant::Pi)
            }
        }
        'O' => {
            let rad_result = value.acos();
            if state.radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(state.precision, rug::float::Constant::Pi)
            }
        }
        'T' => {
            let rad_result = value.atan();
            if state.radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(state.precision, rug::float::Constant::Pi)
            }
        }
        'c' => gaussian_ceil(&value),
        'f' => gaussian_floor(&value),
        'F' => fractional_part(&value),
        'i' => Complex::with_val(state.precision, (value.imag(), 0)),
        'I' => integer_part(&value),
        'l' => value.ln(),
        'L' => value.ln() / Float::with_val(state.precision, state.base).ln(),
        'e' => Complex::with_val(state.precision, (value.real(), 0)),
        'r' => gaussian_round(&value),
        'g' => sign(&value),
        'q' => value.sqrt(),
        's' => {
            if state.radians {
                value.sin()
            } else {
                let pi = Float::with_val(state.precision, rug::float::Constant::Pi);
                (value * pi / Float::with_val(state.precision, 180.0)).sin()
            }
        }
        'o' => {
            if state.radians {
                value.cos()
            } else {
                let pi = Float::with_val(state.precision, rug::float::Constant::Pi);
                (value * pi / Float::with_val(state.precision, 180.0)).cos()
            }
        }
        't' => {
            if state.radians {
                value.tan()
            } else {
                let pi = Float::with_val(state.precision, rug::float::Constant::Pi);
                (value * pi / Float::with_val(state.precision, 180.0)).tan()
            }
        }
        'A' => {
            let rad_result =
                Complex::with_val(state.precision, value.imag().clone().atan2(value.real()));
            if state.radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(state.precision, rug::float::Constant::Pi)
            }
        }

        'x' => {
            // Gaussian error function (erf) approximation
            if !value.imag().is_zero() {
                println!("Warning: complex gaussian error function is likely incorrect!");
            }
            let z = value;
            let one = Complex::with_val(state.precision, 1);
            let two = Complex::with_val(state.precision, 2);
            let pi = Float::with_val(state.precision, std::f64::consts::PI);

            // Series expansion for small |z|
            let erf_series = |z: &Complex| -> Complex {
                let mut sum = z.clone();
                let mut term = z.clone();
                let mut n = Float::with_val(state.precision, 0);
                let threshold =
                    Float::with_val(state.precision, 2).pow(-(state.precision as isize));

                while term.clone().abs().real() > &threshold {
                    n += 1;
                    term = -term.clone() * z * z
                        / Complex::with_val(state.precision, n.clone() * 2 + 1);
                    sum += &term;
                }

                sum * two.clone() / Complex::with_val(state.precision, pi.clone().sqrt())
            };

            // Approximation for larger |z|
            let erf_approx = |z: &Complex| -> Complex {
                let t = Complex::with_val(state.precision, 1)
                    / (Complex::with_val(state.precision, 1)
                        + Complex::with_val(state.precision, 0.3275911) * z.clone().abs());
                let poly = Complex::with_val(state.precision, 0.254829592) * t.clone()
                    - Complex::with_val(state.precision, 0.284496736) * t.clone().pow(2)
                    + Complex::with_val(state.precision, 1.421413741) * t.clone().pow(3)
                    - Complex::with_val(state.precision, 1.453152027) * t.clone().pow(4)
                    + Complex::with_val(state.precision, 1.061405429) * t.pow(5);
                one.clone() - poly * (-z.clone() * z).exp()
            };

            if z.clone().abs().real() < &Float::with_val(state.precision, 0.5) {
                erf_series(&z)
            } else if z.real().clone() >= Float::with_val(state.precision, 0) {
                erf_approx(&z)
            } else {
                -erf_approx(&(-z.clone()))
            }
        }

        _ => return Err(format!("Unknown unary operator: {}", op)),
    };
    debug_println(&format!("Result of unary operation: {}", result));
    Ok(result)
}
/// Applies an operator to the operands on the output queue
///
/// # Arguments
/// * `output_queue` - The queue of operands and intermediate results
/// * `op` - The operator to apply
/// * `precision` - The precision for calculations
/// * `rand_state` - The random state for random number generation
/// * `base` - The current number base
/// * `radians` - Whether to use radians for trigonometric functions
///
/// # Returns
/// * `Ok(())` - If the operation was successful
/// * `Err(String)` - An error message if the operation fails
pub fn apply_binary_operator(output_queue: &mut Vec<Complex>, op: char) -> Result<(), String> {
    debug_println(&format!("Applying binary operator: {}", op));

    if let (Some(b), Some(a)) = (output_queue.pop(), output_queue.pop()) {
        let result = match op {
            '%' => a.modulus(b),
            '^' => a.pow(&b),
            '$' => a.ln() / b.ln(),
            '*' => a * b,
            '+' => a + b,
            '-' => a - b,
            '/' => a / b,
            _ => return Err(format!("Unknown binary operator: {}", op)),
        };
        debug_println(&format!("Result after binary operation: {:?}", result));
        output_queue.push(result);
    } else {
        return Err(format!(
            "Not enough operands for {}!",
            OPERATORS
                .iter()
                .find(|&&(_, symbol, _, _)| symbol == op)
                .map(|(_, _, _, description)| description)
                .unwrap_or(&"unknown operator")
        ));
    }
    Ok(())
}
pub fn gaussian_ceil(z: &Complex) -> Complex {
    Complex::with_val(z.prec(), (z.real().clone().ceil(), z.imag().clone().ceil()))
}
pub fn gaussian_floor(z: &Complex) -> Complex {
    Complex::with_val(
        z.prec(),
        (z.real().clone().floor(), z.imag().clone().floor()),
    )
}
pub fn fractional_part(z: &Complex) -> Complex {
    z - gaussian_floor(z)
}
pub fn integer_part(z: &Complex) -> Complex {
    gaussian_floor(z)
}
pub fn gaussian_round(z: &Complex) -> Complex {
    Complex::with_val(
        z.prec(),
        (z.real().clone().round(), z.imag().clone().round()),
    )
}
pub fn sign(z: &Complex) -> Complex {
    if z.is_zero() {
        z.clone()
    } else {
        z / z.clone().abs()
    }
}
/// Parses a constant from the input
///
/// # Arguments
/// * `input` - The input byte slice
/// * `index` - The starting index in the input
///
/// # Returns
/// * `Ok((Token, usize))` - The parsed constant token and the new index
/// * `Err((String, usize))` - An error message and the position of the error
pub fn parse_constant(
    input: &[u8],
    mut index: usize,
    state: &mut BasecalcState,
) -> Result<(Token, usize), (String, usize)> {
    // Skip leading whitespace
    while index < input.len() && (input[index] == b' ' || input[index] == b'_' || input[index] == b'\t') {
        index += 1;
    }

    // First check for built-in constants
    for &(name, op, _desc) in &CONSTANTS {
        if input[index..]
            .to_ascii_lowercase()
            .starts_with(name.as_bytes())
        {
            return Ok((
                Token {
                    operator: op,
                    ..Token::new()
                },
                index + name.len(),
            ));
        }
    }

    // Then check if this is a variable reference
    if index < input.len() && input[index] == b'@' {
        let mut var_name = String::new();
        let mut curr_index = index + 1;
        
        // Skip whitespace after @
        while curr_index < input.len() && (input[curr_index] == b' ' || input[curr_index] == b'_' || input[curr_index] == b'\t') {
            curr_index += 1;
        }
        
        // Parse variable name, allowing whitespace between characters
        while curr_index < input.len() {
            let c = input[curr_index];
            
            // Skip whitespace within variable name
            if c == b' ' || c == b'_' || c == b'\t' {
                curr_index += 1;
                continue;
            }
            
            if !c.is_ascii_alphanumeric() {
                break;
            }
            
            var_name.push(c.to_ascii_lowercase() as char);
            curr_index += 1;
        }

        if var_name.is_empty() {
            return Err(("Invalid variable name!".to_string(), index));
        }

        // Skip whitespace after variable name
        while curr_index < input.len() && (input[curr_index] == b' ' || input[curr_index] == b'_' || input[curr_index] == b'\t') {
            curr_index += 1;
        }

        // Look for existing variable
        if let Some(pos) = state.variables.iter().position(|v| v.name.to_ascii_lowercase() == var_name) {
            return Ok((
                Token {
                    operator: 'v',
                    var_index: Some(pos),
                    ..Token::new()
                },
                curr_index,
            ));
        }

        // Look ahead for assignment
        let mut look_ahead = curr_index;
        while look_ahead < input.len() && (input[look_ahead] == b' ' || input[look_ahead] == b'_' || input[look_ahead] == b'\t') {
            look_ahead += 1;
        }

        if look_ahead < input.len() && input[look_ahead] == b'=' {
            // This is an assignment - create new variable
            state.variables.push(Variable {
                name: var_name,  // Already lowercase from parsing
                value: Complex::with_val(state.precision, 0),
            });
            return Ok((
                Token {
                    operator: 'v',
                    var_index: Some(state.variables.len() - 1),
                    ..Token::new()
                },
                curr_index,
            ));
        }

        // Variable doesn't exist and this isn't an assignment
        return Err((format!("Undefined variable '{}'!", var_name), index));
    }

    Err((format!("Invalid constant!"), index))
}
/// Parses a number from the input and updates the token
///
/// # Arguments
/// * `input` - The input byte slice
/// * `token` - The token to update with the parsed number
/// * `base` - The current number base
/// * `index` - The starting index in the input
///
/// # Returns
/// * `Ok(usize)` - The new index after parsing the number
/// * `Err((String, usize))` - An error message and the position of the error
pub fn parse_number(
    input: &[u8],
    base: u8,
    mut index: usize,
) -> Result<(Token, usize), (String, usize)> {
    let mut complex = false;
    let mut imaginary = false;
    let mut integer = true;
    let mut expect_sign = true;
    let mut token = Token {
        operator: 1 as char, // 1 denotes number
        ..Token::new()
    };
    while index < input.len()
        && (input[index] == b' ' || input[index] == b'_' || input[index] == b'\t')
    {
        index += 1;
    }

    // Check if we've reached the end of the input after skipping whitespace
    if index >= input.len() {
        return Err(("Incomplete expression!".to_string(), index));
    }
    while index < input.len() {
        let c = input[index];

        if c == b' ' || c == b'_' || c == b'\t' {
            index += 1;
            continue;
        }

        if c == b'[' {
            if !token.real_integer.is_empty() || !token.real_fraction.is_empty() || complex {
                return Err((format!("Unexpected '['!"), index));
            }
            complex = true;
            expect_sign = true;
            index += 1;
            continue;
        }

        if expect_sign {
            if c == b'-' {
                if complex {
                    if imaginary {
                        token.sign.1 = !token.sign.1;
                    } else {
                        token.sign.0 = !token.sign.0;
                    }
                } else {
                    token.sign.0 = !token.sign.0;
                }
                index += 1;
                continue;
            }
        }

        if c == b',' {
            if !complex || imaginary {
                return Err((format!("Unexpected ','!"), index));
            }
            imaginary = true;
            integer = true;
            expect_sign = true;
            index += 1;
            continue;
        }

        if c == b']' {
            if !complex {
                return Err((format!("Unexpected ']'!"), index));
            }

            if token.real_integer.is_empty() && token.real_fraction.is_empty() {
                return Err(("Missing real component!".to_string(), index));
            }
            if token.imaginary_integer.is_empty() && token.imaginary_fraction.is_empty() {
                return Err(("Missing imaginary component!".to_string(), index));
            }
            return Ok((token, index + 1));
        }

        if c == b'.' {
            if !integer {
                return Err((format!("Multiple decimals in number!"), index));
            }
            integer = false;
            index += 1;
            continue;
        }

        let digit = if c.is_ascii_digit() {
            c - b'0'
        } else if c.is_ascii_uppercase() {
            c - b'A' + 10
        } else if c.is_ascii_lowercase() {
            c - b'a' + 10
        } else {
            if token.real_integer.is_empty()
                && token.real_fraction.is_empty()
                && token.imaginary_integer.is_empty()
                && token.imaginary_fraction.is_empty()
            {
                return Err(("Invalid number!".to_string(), index));
            }
            return Ok((token, index));
        };

        if digit >= base {
            let base_char = if base > 9 {
                (base - 10 + b'A') as char
            } else {
                (base + b'0') as char
            };

            if base == 36 {
                return Err((
                    format!(
                        "Digit out of {} (Z+1) range!",
                        get_base_name(base).unwrap().to_ascii_lowercase()
                    ),
                    index,
                ));
            } else {
                return Err((
                    format!(
                        "Digit out of {} ({}) range!",
                        get_base_name(base).unwrap().to_ascii_lowercase(),
                        base_char
                    ),
                    index,
                ));
            };
        }
        expect_sign = false;
        if imaginary {
            if integer {
                token.imaginary_integer.push(digit);
            } else {
                token.imaginary_fraction.push(digit);
            }
        } else {
            if integer {
                token.real_integer.push(digit);
            } else {
                token.real_fraction.push(digit);
            }
        }

        index += 1;
    }

    if complex {
        return Err((format!("Unclosed complex number!"), index));
    }

    if token.real_integer.is_empty()
        && token.real_fraction.is_empty()
        && token.imaginary_integer.is_empty()
        && token.imaginary_fraction.is_empty()
    {
        return Err(("Invalid number!".to_string(), index));
    }

    Ok((token, index))
}
/// Parses an operator from the input
///
/// # Arguments
/// * `input` - The input byte slice
/// * `index` - The starting index in the input
///
/// # Returns
/// * `Ok((Token, usize))` - The parsed operator token and the new index
/// * `Err((String, usize))` - An error message and the position of the error
pub fn parse_operator(input: &[u8], mut index: usize) -> (Token, usize) {
    let mut token = Token::new();

    if index < input.len() {
        // First check for assignment operator
        if input[index] == b'=' {
            token.operator = '=';
            token.operands = 2;
            return (token, index + 1);
        }

        // Then check for other operators
        for &(op_str, op_char, operands, _) in &OPERATORS {
            if input[index..]
                .to_ascii_lowercase()
                .starts_with(op_str.as_bytes())
            {
                token.operator = op_char;
                token.operands = operands;
                index += op_str.len();
                return (token, index);
            }
        }
    }
    (token, index)
}
pub enum CommandResult {
    /// Command was successful, with a message to display
    Success(String),
    /// Command failed, with an error message and the position of the error
    Error(String, usize),
    /// Command was successful but requires no message
    Silent,
    /// `:help`: the front shows its help
    Help,
    /// `:test`: each case of the self test, run on a state of its own
    Tested(Vec<TestOutcome>),
    /// `:dms`: the previous result in dozenal digit names
    Dms(Vec<Span>),
}
/// Why [`tokenize`] gave no tokens: a command ran, or the line is wrong.
pub enum Halt {
    /// A command ran and said this (empty: nothing to say).
    Said(String),
    /// The line is wrong at this byte.
    Wrong(String, usize),
    /// `:help`: the front shows its help.
    Help,
    /// `:test`: each case's outcome.
    Tested(Vec<TestOutcome>),
    /// `:dms`: the previous result in dozenal digit names.
    Dms(Vec<Span>),
}
impl Halt {
    /// The message, for a front that shows only words.
    pub fn message(&self) -> String {
        match self {
            Halt::Said(m) | Halt::Wrong(m, _) => m.clone(),
            Halt::Help => String::new(),
            Halt::Tested(outcomes) => {
                let passed = outcomes.iter().filter(|o| o.passed()).count();
                format!("{}/{} tests passed.", passed, outcomes.len())
            }
            Halt::Dms(spans) => spans_text(spans),
        }
    }
}
/// Parses a command from the input and updates calculator settings
///
/// # Arguments
/// * `input` - The input byte slice
/// * `index` - The starting index in the input
/// * `base` - The current number base
/// * `precision` - The current precision for calculations
/// * `digits` - The number of digits to display in results
/// * `radians` - Whether to use radians for trigonometric functions
/// * `colours` - The colour scheme for output formatting
/// * `rand_state` - The random state for random number generation
/// * `prev_result` - The previous calculation result
///
/// # Returns
/// * `CommandResult::Success(String)` - Command was successful, with a message to display
/// * `CommandResult::Error(String, usize)` - Command failed, with an error message and the position of the error
/// * `CommandResult::Silent` - Command was successful but requires no message (like :help)
pub fn parse_command(input: &[u8], mut index: usize, state: &mut BasecalcState) -> CommandResult {
    match &input[index..] {
        s if s.eq_ignore_ascii_case(b"test") => {
            CommandResult::Tested(run_tests())
        }
        s if s.len() >= 4 && s[..4].eq_ignore_ascii_case(b"base") => {
            index += 4;
            // Skip whitespace
            while index < input.len()
                && (input[index] == b' ' || input[index] == b'_' || input[index] == b'\t')
            {
                index += 1;
            }

            if index >= input.len() {
                return CommandResult::Error("Missing base value!".to_string(), index);
            }

            let digit = input[index];
            let new_base = if digit.is_ascii_digit() {
                digit - b'0'
            } else if digit.is_ascii_uppercase() {
                digit - b'A' + 10
            } else if digit.is_ascii_lowercase() {
                digit - b'a' + 10
            } else {
                return CommandResult::Error("Invalid base value!".to_string(), index);
            };
            if new_base == 1 || new_base > 36 {
                return CommandResult::Error(
                    "Base must be between 2 and 36!\nUse ':base 0' for base 36 (Z+1)".to_string(),
                    index,
                );
            }
            state.base = if new_base == 0 { 36 } else { new_base };

            let base_char = match state.base {
                0..=9 => (state.base as u8 + b'0') as char,
                10..=35 => (state.base as u8 - 10 + b'A') as char,
                36 => 'Z',
                _ => '?',
            };

            state.set_precision();
            let message = match get_base_name(state.base) {
                Some(name) => {
                    if state.base == 36 {
                        format!("Base set to {} (Z+1).", name)
                    } else {
                        format!("Base set to {} ({}).", name, base_char)
                    }
                }
                None => format!("Base set to {}, unsupported base name.", base_char),
            };

            // Check for any trailing characters
            index += 1;
            while index < input.len() {
                if input[index] != b' ' && input[index] != b'_' && input[index] != b'\t' {
                    return CommandResult::Error(
                        "Invalid characters after base value!".to_string(),
                        index,
                    );
                }
                index += 1;
            }
            CommandResult::Success(message)
        }
        s if s.len() >= 6 && s[..6].eq_ignore_ascii_case(b"digits") => {
            let token = Token::new();
            let value;
            let new_index;
            match parse_number(input, state.base, index + 6) {
                Ok((token, x)) => {
                    new_index = x;
                    if token.real_fraction.len() > 0
                        || token.imaginary_integer.len() > 0
                        || token.imaginary_fraction.len() > 0
                        || token.sign.0
                    {
                        return CommandResult::Error(
                            "Precision must be a positive real integer!".to_string(),
                            index,
                        );
                    }

                    value = token2num(&token, state).real().clone().round().to_f64() as usize;
                    if value == 0 {
                        return CommandResult::Error(
                            "Precision must be a positive real integer!".to_string(),
                            index,
                        );
                    }
                }
                Err((msg, pos)) => {
                    return CommandResult::Error(msg, pos);
                }
            }
            index = new_index;

            // Check if there's anything after the number
            if index < input.len() {
                for i in index..input.len() {
                    if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                        return CommandResult::Error(
                            "Invalid characters after digits value!".to_string(),
                            i,
                        );
                    }
                }
            }
            state.digits = value;
            state.set_precision();
            if token.imaginary_integer.len() > 0 || token.imaginary_fraction.len() > 0 {
                return CommandResult::Error(
                    "Precision must be a real integer!".to_string(),
                    index,
                );
            }
            CommandResult::Success(format!(
                "Precision set to {} digits.",
                format_int(value, state.base as usize)
            ))
        }
        s if s.len() >= 7 && s[..7].eq_ignore_ascii_case(b"degrees") => {
            // Check if there's anything after the command
            for i in index + 7..input.len() {
                if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                    return CommandResult::Error(
                        "Invalid characters after command!".to_string(),
                        i,
                    );
                }
            }
            state.radians = false;
            CommandResult::Success("Angle units set to degrees.".to_string())
        }
        s if s.len() >= 7 && s[..7].eq_ignore_ascii_case(b"radians") => {
            // Check if there's anything after the command
            for i in index + 7..input.len() {
                if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                    return CommandResult::Error(
                        "Invalid characters after command!".to_string(),
                        i,
                    );
                }
            }
            state.radians = true;
            CommandResult::Success("Angle units set to radians.".to_string())
        }
        s if s.len() >= 3 && s[..3].eq_ignore_ascii_case(b"dms") => {
            // Check if there's anything after the command
            for i in index + 3..input.len() {
                if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                    return CommandResult::Error(
                        "Invalid characters after command!".to_string(),
                        i,
                    );
                }
            }
            CommandResult::Dms(num2dms(&state.prev_result, state))
        }
        s if s.eq_ignore_ascii_case(b"help") => CommandResult::Help,
        s if s.len() >= 5 && s[..5].eq_ignore_ascii_case(b"debug") => {
            // Toggle debug mode
            let new_state = !DEBUG.load(Ordering::Relaxed);
            DEBUG.store(new_state, Ordering::Relaxed);
            CommandResult::Success(format!(
                "Debug {}",
                if new_state { "enabled" } else { "disabled" }
            ))
        }
        _ => CommandResult::Error("Unknown command!".to_string(), index),
    }
}
pub fn generate_random(precision: u32, rand_state: &mut rug::rand::RandState) -> Complex {
    let real = Float::with_val(precision, Float::random_cont(rand_state));
    Complex::with_val(precision, (real, 0))
}
pub fn gaussian_complex_random(precision: u32, rand_state: &mut rug::rand::RandState) -> Complex {
    // Box-Muller transform to generate Gaussian random numbers
    let u1 = Float::with_val(precision, Float::random_cont(rand_state));
    let u2 = Float::with_val(precision, Float::random_cont(rand_state));

    let two = Float::with_val(precision, 2);
    let pi = Float::with_val(precision, rug::float::Constant::Pi);

    let r = (Float::with_val(precision, -two.clone() * u1.ln())).sqrt();
    let theta = two * pi * u2;

    let real = &r * theta.clone().cos();
    let imag = &r * theta.sin();

    Complex::with_val(precision, (real, imag))
}
/// Converts a token to a complex number
///
/// # Arguments
/// * `token` - The token to convert
/// * `state` - The current calculator state
///
/// # Returns
/// * `Complex` - The complex number representation of the token
pub fn token2num(token: &Token, state: &mut BasecalcState) -> Complex {
    match token.operator {
        // User-defined constants
        'v' => {
            if let Some(index) = token.var_index {
                state.variables[index].value.clone()
            } else {
                Complex::with_val(state.precision, 0)
            }
        }
        // Built-in constants
        'E' => Complex::with_val(state.precision, Float::with_val(state.precision, 1).exp()),
        'G' => Complex::with_val(state.precision, rug::float::Constant::Euler),
        'C' => Complex::with_val(state.precision, rug::float::Constant::Catalan),
        'p' => Complex::with_val(state.precision, rug::float::Constant::Pi),
        'P' => {
            let prec = state.precision;
            let one = Float::with_val(prec, 1);
            let five = Float::with_val(prec, 5);
            let sqrt5 = five.sqrt();
            Complex::with_val(prec, (one + sqrt5) / 2)
        }
        'r' => generate_random(state.precision, &mut state.rand_state),
        'g' => gaussian_complex_random(state.precision, &mut state.rand_state),
        '&' => state.prev_result.clone(),

        // Regular numbers
        _ => {
            let mut real_int = Float::with_val(state.precision, 0);
            for &digit in &token.real_integer {
                real_int *= state.base;
                real_int += digit;
            }
            let mut real_frac = Float::with_val(state.precision, 0);
            for &digit in token.real_fraction.iter().rev() {
                real_frac += digit as f64;
                real_frac /= state.base as f64;
            }

            let mut imag_int = Float::with_val(state.precision, 0);
            for &digit in &token.imaginary_integer {
                imag_int *= state.base;
                imag_int += digit;
            }
            let mut imag_frac = Float::with_val(state.precision, 0);
            for &digit in token.imaginary_fraction.iter().rev() {
                imag_frac += digit as f64;
                imag_frac /= state.base as f64;
            }

            let mut real = Float::with_val(state.precision, &real_int + &real_frac);
            let mut imaginary = Float::with_val(state.precision, &imag_int + &imag_frac);

            if token.sign.0 {
                real = -real;
            }
            if token.sign.1 {
                imaginary = -imaginary;
            }

            Complex::with_val(state.precision, (real, imaginary))
        }
    }
}
/// Converts a complex number to a vector of role-tagged spans for display
///
/// # Arguments
/// * `num` - The complex number to convert
/// * `base` - The current number base
/// * `digits` - The number of digits to display
/// * `colours` - The colour scheme for output formatting
///
/// # Returns
/// * `Vec<Span>` - A vector of role-tagged spans representing the number
pub fn num2string(num: &Complex, state: &BasecalcState) -> Vec<Span> {
    let mut result = Vec::new();

    if num.real().is_nan()
        || num.imag().is_nan()
        || num.real().is_infinite()
        || num.imag().is_infinite()
    {
        result.push(span(Role::Nan, "NaN"));
        return result;
    }

    if num.imag().is_zero() {
        result.push(span(Role::Plain, " "));
        result.extend(format_part(num.real(), state, true, true));
    } else {
        result.push(span(Role::Brackets, "["));
        result.extend(format_part(num.real(), state, true, false));
        result.push(span(Role::Comma, " ,"));
        result.extend(format_part(num.imag(), state, false, false));
        result.push(span(Role::Brackets, " ]"));
    }

    result
}
/// Converts a complex number to a vector of DMS role-tagged spans for display
///
/// # Arguments
/// * `num` - The complex number to convert
/// * `base` - The current number base
/// * `digits` - The number of digits to display
/// * `colours` - The colour scheme for output formatting
///
/// # Returns
/// * `Vec<Span>` - A vector of role-tagged spans representing the number
pub fn num2dms(num: &Complex, state: &BasecalcState) -> Vec<Span> {
    let mut result = Vec::new();

    if num.real().is_nan()
        || num.imag().is_nan()
        || num.real().is_infinite()
        || num.imag().is_infinite()
    {
        result.push(span(Role::Nan, "NaN"));
        return result;
    }

    if num.imag().is_zero() {
        result.push(span(Role::Plain, " "));
        result.extend(format_dms(num.real(), state, true, true));
    } else {
        result.push(span(Role::Brackets, "["));
        result.extend(format_dms(num.real(), state, true, false));
        result.push(span(Role::Comma, " ,"));
        result.extend(format_dms(num.imag(), state, false, false));
        result.push(span(Role::Brackets, " ]"));
    }

    result
}
/// Formats a part of a complex number (real or imaginary) as a vector of role-tagged spans
///
/// # Arguments
/// * `num` - The float number to format
/// * `base` - The current number base
/// * `num_digits` - The number of digits to display
/// * `colours` - The colour scheme for output formatting
/// * `is_real` - Whether this is the real part of a complex number
/// * `is_lone` - Whether this is a standalone number (not part of a complex number)
///
/// # Returns
/// * `Vec<Span>` - A vector of role-tagged spans representing the formatted number
pub fn format_part(
    num: &rug::Float,
    state: &BasecalcState,
    is_real: bool,
    is_lone: bool,
) -> Vec<Span> {
    let mut result = Vec::new();

    if num.is_zero() {
        result.push(span(Role::Plain, " "));
        result.push(span(Role::LoneInteger, "0"));
        result.push(span(Role::Decimal, "."));
        return result;
    }
    if num.is_nan() || num.is_infinite() {
        result.push(span(Role::Nan, "NaN"));
        return result;
    }

    let is_positive = num.is_sign_positive();
    if is_positive {
        result.push(span(Role::Plain, " "));
    } else {
        result.push(span(Role::Sign, "-"));
    }

    let mut num_abs = num.clone().abs();
    let mut decimal_place = (num_abs.clone().log2()
        / (Float::with_val(num.prec(), state.base)).log2())
    .floor()
    .to_f64() as isize;
    num_abs = num_abs / (Float::with_val(num.prec(), state.base)).pow(decimal_place);
    num_abs += (Float::with_val(num.prec(), state.base)).pow(-(state.digits as isize - 1)) / 2;
    if num_abs > state.base {
        num_abs = num.clone().abs();
        decimal_place += 1;
        num_abs = num_abs / (Float::with_val(num.prec(), state.base)).pow(decimal_place);
        num_abs += (Float::with_val(num.prec(), state.base)).pow(-(state.digits as isize - 1)) / 2;
    }

    let mut integer_part = String::new();
    let mut decimal = false;
    let mut place = 0;
    let mut offset = place as isize - decimal_place;
    while offset <= 0 && place < state.digits {
        place += 1;
        let digit: u8 = num_abs.clone().floor().cast();
        num_abs = num_abs - digit;
        num_abs *= state.base;
        let digit_char = if digit < 10 {
            (digit + b'0') as char
        } else {
            ((digit - 10) + b'A') as char
        };
        integer_part.push(digit_char);
        offset = place as isize - decimal_place;
        if offset.rem_euc(3) == 1 && offset != 1 {
            //&& place != num_digits - 1
            integer_part.push(' ')
        }
    }
    if offset == 1 {
        decimal = true;
    }
    let mut fractional_part = String::new();
    while offset > 0 && place < state.digits {
        place += 1;
        let digit: u8 = num_abs.clone().floor().cast();
        num_abs = num_abs - digit;
        num_abs *= state.base;
        let digit_char = if digit < 10 {
            (digit + b'0') as char
        } else {
            ((digit - 10) + b'A') as char
        };
        fractional_part.push(digit_char);
        offset = place as isize - decimal_place;
        if offset.rem_euc(3) == 1 {
            //} && place != num_digits - 1 {
            fractional_part.push(' ')
        }
    }
    let (int_role, frac_role) = if is_lone {
        (Role::LoneInteger, Role::LoneFraction)
    } else if is_real {
        (Role::RealInteger, Role::RealFraction)
    } else {
        (Role::ImaginaryInteger, Role::ImaginaryFraction)
    };
    let prec = num_abs.prec();
    let tilde = (num_abs * Float::with_val(prec, 2) - Float::with_val(prec, state.base)).abs()
        > 2f64.pow(-16);
    if decimal {
        if integer_part.is_empty() {
            result.push(span(int_role, "0"));
        } else {
            result.push(span(int_role, integer_part));
        }
        result.push(span(Role::Decimal, "."));
        result.push(span(frac_role, trim_zeros(fractional_part)));
        if tilde {
            result.push(span(Role::Tilde, "~"));
        } else {
            result.push(span(Role::Plain, " "));
        }
    } else {
        if integer_part.is_empty() {
            let mut number = trim_zeros(fractional_part);
            let first = number.as_bytes()[0];
            let is_space = first == b' ';
            if is_space {
                let mut new_number = "".to_owned();
                new_number.push(number.as_bytes()[1] as char);
                new_number.push('.');
                new_number.push_str(number.split_at(2).1);
                number = new_number;
            } else {
                let mut new_number = "".to_owned();
                new_number.push(first as char);
                new_number.push('.');
                new_number.push_str(number.split_at(1).1);
                number = new_number;
            }
            result.push(span(frac_role, number));
            if tilde {
                result.push(span(Role::Tilde, "~"));
            } else {
                result.push(span(Role::Plain, " "));
            }
            result.push(span(Role::Colon, " :"));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, state.base as usize));
                result.push(span(Role::Exponent, exponent));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, state.base as usize));
                result.push(span(Role::Exponent, exponent));
            }
        } else {
            let mut number = trim_zeros(integer_part);
            let first = number.as_bytes()[0];
            let is_space = first == b' ';
            if is_space {
                let mut new_number = "".to_owned();
                new_number.push(number.as_bytes()[1] as char);
                new_number.push('.');
                new_number.push_str(number.split_at(2).1);
                number = new_number;
            } else {
                let mut new_number = "".to_owned();
                new_number.push(first as char);
                new_number.push('.');
                new_number.push_str(number.split_at(1).1);
                number = new_number;
            }
            result.push(span(int_role, number));
            if tilde {
                result.push(span(Role::Tilde, "~"));
            } else {
                result.push(span(Role::Plain, " "));
            }
            result.push(span(Role::Colon, " :"));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, state.base as usize));
                result.push(span(Role::Exponent, exponent));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, state.base as usize));
                result.push(span(Role::Exponent, exponent));
            }
        }
    }
    result
}
/// Formats a part of a complex number (real or imaginary) as a vector of role-tagged spans
///
/// # Arguments
/// * `num` - The float number to format
/// * `base` - The current number base
/// * `num_digits` - The number of digits to display
/// * `colours` - The colour scheme for output formatting
/// * `is_real` - Whether this is the real part of a complex number
/// * `is_lone` - Whether this is a standalone number (not part of a complex number)
///
/// # Returns
/// * `Vec<Span>` - A vector of role-tagged spans representing the formatted DMS part
pub fn format_dms(
    num: &rug::Float,
    state: &BasecalcState,
    is_real: bool,
    is_lone: bool,
) -> Vec<Span> {
    let mut result = Vec::new();

    if num.is_zero() {
        result.push(span(Role::Plain, " "));
        result.push(span(Role::LoneInteger, "Zil"));
        result.push(span(Role::Decimal, "."));
        return result;
    }
    if num.is_nan() || num.is_infinite() {
        result.push(span(Role::Nan, "NaN"));
        return result;
    }

    let is_positive = num.is_sign_positive();
    if is_positive {
        result.push(span(Role::Plain, " "));
    } else {
        result.push(span(Role::Sign, "-"));
    }

    let mut num_abs = num.clone().abs();
    let mut decimal_place = (num_abs.clone().log2() / (Float::with_val(num.prec(), 12)).log2())
        .floor()
        .to_f64() as isize;
    num_abs = num_abs / (Float::with_val(num.prec(), 12)).pow(decimal_place);
    num_abs += (Float::with_val(num.prec(), 12)).pow(-(state.digits as isize - 1)) / 2;
    if num_abs > 12 {
        num_abs = num.clone().abs();
        decimal_place += 1;
        num_abs = num_abs / (Float::with_val(num.prec(), 12)).pow(decimal_place);
        num_abs += (Float::with_val(num.prec(), 12)).pow(-(state.digits as isize - 1)) / 2;
    }

    let mut integer_part = String::new();
    let mut decimal = false;
    let mut place = 0;
    let mut offset = place as isize - decimal_place;
    while offset <= 0 && place < state.digits {
        place += 1;
        let digit: u8 = num_abs.clone().floor().cast();
        num_abs = num_abs - digit;
        num_abs *= 12;
        let name = match digit {
            0 => "Zil",
            1 => "Zila",
            2 => "Zilor",
            3 => "Ter",
            4 => "Tera",
            5 => "Teror",
            6 => "Lun",
            7 => "Luna",
            8 => "Lunor",
            9 => "Stel",
            10 => "Stela",
            11 => "Stelor",
            _ => "NaN",
        };
        integer_part.extend(name.chars());
        offset = place as isize - decimal_place;
        if offset.rem_euc(3) == 1 && offset != 1 {
            //&& place != num_digits - 1
            integer_part.push(' ')
        }
    }
    if offset == 1 {
        decimal = true;
    }
    let mut fractional_part = String::new();
    while offset > 0 && place < state.digits {
        place += 1;
        let digit: u8 = num_abs.clone().floor().cast();
        num_abs = num_abs - digit;
        num_abs *= 12;
        let name = match digit {
            0 => "Zil",
            1 => "Zila",
            2 => "Zilor",
            3 => "Ter",
            4 => "Tera",
            5 => "Teror",
            6 => "Lun",
            7 => "Luna",
            8 => "Lunor",
            9 => "Stel",
            10 => "Stela",
            11 => "Stelor",
            _ => "NaN",
        };
        fractional_part.extend(name.chars());
        offset = place as isize - decimal_place;
        if offset.rem_euc(3) == 1 {
            //} && place != num_digits - 1 {
            fractional_part.push(' ')
        }
    }
    let (int_role, frac_role) = if is_lone {
        (Role::LoneInteger, Role::LoneFraction)
    } else if is_real {
        (Role::RealInteger, Role::RealFraction)
    } else {
        (Role::ImaginaryInteger, Role::ImaginaryFraction)
    };
    let prec = num_abs.prec();
    let tilde =
        (num_abs * Float::with_val(prec, 2) - Float::with_val(prec, 12)).abs() > 2f64.pow(-16);
    if decimal {
        if integer_part.is_empty() {
            result.push(span(int_role, "Zil"));
        } else {
            result.push(span(int_role, integer_part));
        }
        result.push(span(Role::Decimal, "."));
        result.push(span(frac_role, trim_zeros(fractional_part)));
        if tilde {
            result.push(span(Role::Tilde, "~"));
        } else {
            result.push(span(Role::Plain, " "));
        }
    } else {
        if integer_part.is_empty() {
            let mut number = trim_zeros(fractional_part);
            let first = number.as_bytes()[0];
            let is_space = first == b' ';
            if is_space {
                let mut new_number = "".to_owned();
                new_number.push(number.as_bytes()[1] as char);
                new_number.push('.');
                new_number.push_str(number.split_at(2).1);
                number = new_number;
            } else {
                let mut new_number = "".to_owned();
                new_number.push(first as char);
                new_number.push('.');
                new_number.push_str(number.split_at(1).1);
                number = new_number;
            }
            result.push(span(frac_role, number));
            if tilde {
                result.push(span(Role::Tilde, "~"));
            } else {
                result.push(span(Role::Plain, " "));
            }
            result.push(span(Role::Colon, " :"));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, 12 as usize));
                result.push(span(Role::Exponent, exponent));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, 12 as usize));
                result.push(span(Role::Exponent, exponent));
            }
        } else {
            let mut number = trim_zeros(integer_part);
            let first = number.as_bytes()[0];
            let is_space = first == b' ';
            if is_space {
                let mut new_number = "".to_owned();
                new_number.push(number.as_bytes()[1] as char);
                new_number.push('.');
                new_number.push_str(number.split_at(2).1);
                number = new_number;
            } else {
                let mut new_number = "".to_owned();
                new_number.push(first as char);
                new_number.push('.');
                new_number.push_str(number.split_at(1).1);
                number = new_number;
            }
            result.push(span(int_role, number));
            if tilde {
                result.push(span(Role::Tilde, "~"));
            } else {
                result.push(span(Role::Plain, " "));
            }
            result.push(span(Role::Colon, " :"));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, 12 as usize));
                result.push(span(Role::Exponent, exponent));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, 12 as usize));
                result.push(span(Role::Exponent, exponent));
            }
        }
    }
    result
}
pub fn trim_zeros(mut number: String) -> String {
    let mut index = number.len();
    while index > 0 {
        if number.as_bytes()[index - 1] != b'0' && number.as_bytes()[index - 1] != b' ' {
            break;
        }
        index -= 1;
    }
    number.truncate(index);
    number
}
/// Formats an integer in the specified base as a string
///
/// # Arguments
/// * `num` - The integer to format
/// * `base` - The base to use for formatting (2 to 36)
///
/// # Returns
/// * `String` - The formatted integer as a string
///
/// # Notes
/// - For bases > 10, uses uppercase letters A-Z for digits 10-35
/// - Returns "0" if the input is 0
/// - Does not handle negative numbers
pub fn format_int(mut num: usize, base: usize) -> String {
    if num == 0 {
        return "0".to_owned();
    }
    let mut number = "".to_owned();
    while num != 0 {
        let mut digit = (num % base) as u8;
        num = num / base;
        if digit < 10 {
            digit += b'0'
        } else {
            digit += b'A' - 10
        }
        number.push(digit as char);
    }
    number.chars().rev().collect()
}
pub fn get_base_name(base: u8) -> Option<&'static str> {
    match base {
        2 => Some("Binary"),
        3 => Some("Ternary"),
        4 => Some("Quaternary"),
        5 => Some("Quinary"),
        6 => Some("Senary"),
        7 => Some("Septenary"),
        8 => Some("Octal"),
        9 => Some("Nonary"),
        10 => Some("Decimal"),
        11 => Some("Undecimal"),
        12 => Some("Dozenal"),
        13 => Some("Tridecimal"),
        14 => Some("Tetradecimal"),
        15 => Some("Pentadecimal"),
        16 => Some("Hexadecimal"),
        17 => Some("Heptadecimal"),
        18 => Some("Octodecimal"),
        19 => Some("Enneadecimal"),
        20 => Some("Vigesimal"),
        21 => Some("Unvigesimal"),
        22 => Some("Duovigesimal"),
        23 => Some("Trivigesimal"),
        24 => Some("Tetravigesimal"),
        25 => Some("Pentavigesimal"),
        26 => Some("Hexavigesimal"),
        27 => Some("Heptavigesimal"),
        28 => Some("Octovigesimal"),
        29 => Some("Enneabigesimal"),
        30 => Some("Trigesimal"),
        31 => Some("Untrigesimal"),
        32 => Some("Duotrigesimal"),
        33 => Some("Tritrigesimal"),
        34 => Some("Tetratrigesimal"),
        35 => Some("Pentatrigesimal"),
        36 => Some("Hexatrigesimal"),
        _ => None,
    }
}
pub fn debug_println(msg: &str) {
    if DEBUG.load(Ordering::Relaxed) {
        println!("{}", msg);
    }
}
pub fn run_tests() -> Vec<TestOutcome> {
    let mut state = BasecalcState::new();
    let tests = vec![
        (":baSE C", "Base set to Dozenal (C)."),
        (":DIGits    \t__\t\t2  0", "Precision set to 20 digits."),
        // (":debug", "Debug enabled"),
        (
            "---1+2*(3+4*(5+6))^(-1/0.3)",
            " -0.BBB BBA 939 245 70A 7B2 93B B06~",
        ),
        ("5^-25", "  1.86 BA3 547 200 980 95A 405 483~ :-17"),
        ("(1+2)*3", "  9."),
        ("--1+2*3", "  7."),
        ("(1+2)*(3+4)", "  19."),
        ("1+2*(3+4)", "  13."),
        ("((1+2)*3)+4", "  11."),
        ("1+(2*3)+4", "  B."),
        ("2^(3^2)", "  368."),
        ("(2^3)^2", "  54."),
        ("1/(1+1/(1+1/(1+1/2)))", "  0.76"),
        ("(((1+2)+3)+4)", "  A."),
        ("1+(2+(3+4))", "  A."),
        ("(1+2+3+4)", "  A."),
        ("1 2 + 3", "  15."),
        ("-3", " -3."),
        ("--3", "  3."),
        ("---3", " -3."),
        ("----3", "  3."),
        ("1-3", " -2."),
        ("1--3", "  4."),
        ("1---3", " -2."),
        ("1----3", "  4."),
        ("1/3+1/3+1/3-1", "  0."),
        ("1 2 3 4 5", "  12 345."),
        (
            "5^-25*[-3.24,-4.1b]",
            "[-5.58 BA6 424 28A 6A9 238 829 27A~ :-17 ,-7.17 49A 618 591 429 757 6B6 512~ :-17 ]",
        ),
        ("#sqrt-1", "[ 0. , 1.  ]"),
        (
            "#sqrt(#sqrt-1)",
            "[ 0.859 A69 650 3BA 297 996 256 428~ , 0.859 A69 650 3BA 297 996 256 428~ ]",
        ),
        (
            "#sqrt#sqrt-1",
            "[ 0.859 A69 650 3BA 297 996 256 428~ , 0.859 A69 650 3BA 297 996 256 428~ ]",
        ),
        ("#sqrt(-1-1)", "[ 0. , 1.4B7 917 0A0 7B8 573 770 4B0 85~ ]"),
        ("#sqrt-1-1", "[-1.  , 1.  ]"),
        ("-#sIn(@pi/2)", " -1."),
        ("#sin(@pi/4)", "  0.859 A69 650 3BA 297 996 256 428~"),
        (":deGreEs", "Angle units set to degrees."),
        ("#sin76", "  1."), // In degrees
        (":radiAns", "Angle units set to radians."),
        ("#sin76", "  0.A88 9AB 897 724 376 B81 A25 541~"), // In radians
        ("#sin#cos@pi", " -0.A12 08A A92 234 12B 470 074 934~"),
        ("-#cos#sin0", " -1."),
        ("#cos-#sin0", "  1."),
        ("#cos#sin-0", "  1."),
        ("---#cos---@pi", "  1."),
        ("#log(100)/2", "  1."),
        ("(@pi+@e)^2", "  2A.408 353 754 8B8 38B 235 632 3~"),
        ("#sqrt(1+2+3)+)", "Mismatched parentheses!"),
        ("[12,34.56,]", "Unexpected ','!"),
        ("[12, 34. 56,", "Unexpected ','!"),
        ("[ 12 ,34.56", "Unclosed complex number!"),
        ("[-12.,34.56[1,2]]", "Unexpected '['!"),
        ("[ 1 2..,34.56]", "Multiple decimals in number!"),
        ("[,1234.56 ]", "Missing real component!"),
        ("( (())1+2 ( ()))", "Expected number!"),
        ("(1+2))", "Mismatched parentheses!"),
        ("(1+2", "Mismatched parentheses!"),
        ("1+*2", "Invalid number!"),
        (" #sin()", "Expected number!"),
        ("#sin", "Incomplete expression!"),
        ("#sin(#cos())", "Expected number!"),
        ("1/0", "NaN"),
        ("[0,-1]/0", "NaN"),
        ("1.2.3", "Multiple decimals in number!"),
        ("(  1+2)*(3+4", "Mismatched parentheses!"),
        ("#log(0)", "NaN"),
        ("@pi@e", "Invalid operator!"),
        ("#sin()#cos ( )", "Expected number!"),
        ("1++2", "Invalid number!"),
        ("((1  + 2  ) *3", "Mismatched parentheses!"),
        ("1+(2*3", "Mismatched parentheses!"),
        ("1 2 3 +", "Incomplete expression!"),
        ("1 *  + 2", "Invalid number!"),
        ("#funky(1)", "Invalid number!"),
        ("1 / (2-2)", "NaN"),
        ("(((1+2)*(3+4))+5", "Mismatched parentheses!"),
        ("*1", "Invalid number!"),
        ("1*", "Incomplete expression!"),
        ("()", "Expected number!"),
        ("#sin", "Incomplete expression!"),
        ("12345 678 9abcdef", "Digit out of dozenal (C) range!"),
        ("7", "  7."),
        ("&", "  7."),
        ("&+&", "  12."),
        (":BaSe0", "Base set to Hexatrigesimal (Z+1)."),
        ("#aCoS#SiGn1", "  0."),
        ("#aCoS(#SiGn1)", "  0."),
        (
            "#aCoS#SiGn[1,2]",
            "[ 1.8MV CO2 534 S9U VVE RVY UOO 25~ ,-0.UBU UDT BMM E9G 8UA I4H 8G8 32J~ ]",
        ),
        (
            "#aCoS(#SiGn[1,2])",
            "[ 1.8MV CO2 534 S9U VVE RVY UOO 25~ ,-0.UBU UDT BMM E9G 8UA I4H 8G8 32J~ ]",
        ),
        ("#aCoS#SiGn#sin(@pi/2)", "  0."),
        ("#aCoS#SiGn#sin(@pi/2)", "  0."),
        (
            "#abs(-3*g)+#sqrt(y)/5",
            "  1D.5ZD S0P CPH DKF GU1 V0S NUV S~",
        ),
        // Complex nested functions with constants
        ("#sin#cos#tan3^2+1", "  1.P5N M5R ZCQ 6RZ NW6 FIS 23Y NV~"),
        ("@1=4+1", "@1 =   5."),
        ("5/@1", "  1."),
    ];
    let mut outcomes = Vec::new();
    for (input, expected) in tests {
        let shown = match line(input, &mut state) {
            Ok(spans) => spans,
            Err(Halt::Said(msg)) | Err(Halt::Wrong(msg, _)) => vec![span(Role::Message, msg)],
            Err(other) => vec![span(Role::Message, other.message())],
        };
        outcomes.push(TestOutcome { input, expected, shown });
    }
    outcomes
}
/// One case of the self test: the line, what it should show, and what it showed.
pub struct TestOutcome {
    pub input: &'static str,
    pub expected: &'static str,
    pub shown: Vec<Span>,
}
impl TestOutcome {
    /// What it showed as the test compares it.
    pub fn got(&self) -> String {
        coloured_vec_to_string(&self.shown)
    }
    pub fn passed(&self) -> bool {
        self.got() == self.expected
    }
}
/// One line as the terminal runs it: a command, or an expression evaluated and its result written (`@name = ` before an assignment's), the result kept as the previous one (`&`).
/// An evaluation error comes back as a lone [`Role::Error`] span, as the terminal shows it.
pub fn line(input: &str, state: &mut BasecalcState) -> Result<Vec<Span>, Halt> {
    let tokens = tokenize(input, state)?;
    Ok(match evaluate_tokens(&tokens, state) {
        Ok(result) => {
            let spans = result_spans(&result, state);
            state.prev_result = result.value;
            spans
        }
        Err(err) => vec![span(Role::Error, err)],
    })
}
/// One expression as a keypad front runs it: its value, kept as the previous result (`&`), and its spans; or the words of why not.
/// Commands (`:base` and the rest) are the terminal's: a front with controls of its own sets the state's fields and calls [`BasecalcState::set_precision`], as [`parse_command`] does.
pub fn evaluate(input: &str, state: &mut BasecalcState) -> Result<(Complex, Vec<Span>), String> {
    let tokens = tokenize(input, state).map_err(|halt| halt.message())?;
    let result = evaluate_tokens(&tokens, state)?;
    let spans = result_spans(&result, state);
    state.prev_result = result.value.clone();
    Ok((result.value, spans))
}
/// A result as the terminal writes it: `@name = ` before an assignment's value.
pub fn result_spans(result: &EvalResult, state: &BasecalcState) -> Vec<Span> {
    let mut spans = Vec::new();
    if let Some(var_idx) = result.assignment {
        spans.push(span(Role::Message, format!("@{} = ", state.variables[var_idx].name)));
    }
    spans.extend(num2string(&result.value, state));
    spans
}
/// The spans' text as the self test compares it: ASCII only, trailing space trimmed.
pub fn coloured_vec_to_string(coloured_vec: &[Span]) -> String {
    let mut result = String::new();
    for c in spans_text(coloured_vec).chars() {
        if c.is_ascii() {
            result.push(c);
        }
    }
    result.trim_end().to_owned()
}
#[cfg(test)]
mod tests {
    /// The terminal's own self test (`:test`), every case.
    #[test]
    fn self_test_passes() {
        let outcomes = super::run_tests();
        let failed: Vec<String> = outcomes.iter().filter(|o| !o.passed()).map(|o| format!("{}: expected {:?}, got {:?}", o.input, o.expected, o.got())).collect();
        assert!(failed.is_empty(), "{failed:#?}");
    }

    /// A keypad front's call: the value kept for `&`, an error in words, and the state's own fields for the base.
    #[test]
    fn evaluate_keeps_the_previous_result() {
        let mut state = super::BasecalcState::new();
        let (value, spans) = super::evaluate("6*7", &mut state).unwrap();
        assert_eq!(value.real().to_f64(), 42.);
        assert_eq!(super::coloured_vec_to_string(&spans), "  42.");
        state.base = 16;
        state.set_precision();
        let (_, spans) = super::evaluate("&+1", &mut state).unwrap();
        assert_eq!(super::coloured_vec_to_string(&spans), "  2B.");
        assert_eq!(super::evaluate("1+", &mut state).unwrap_err(), "Incomplete expression!");
        assert_eq!(state.prev_result.real().to_f64(), 43.);
    }
}
