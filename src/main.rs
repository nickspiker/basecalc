// Basecalc: Your Towel in the Mathematical Cosmos
// Copyright (C) 2024 Nick Spiker
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
use az::Cast;
use colored::*;
use rug::ops::*;
use rug::*;
use rustyline::{error::ReadlineError, Config, DefaultEditor};
use std::sync::atomic::{AtomicBool, Ordering};
fn main() -> rustyline::Result<()> {
    let config = Config::builder().build();
    let mut rl = DefaultEditor::with_config(config)?;

    let mut base = 10;
    let mut digits = 12;
    let mut precision = (digits as f64 * (base as f64).log2()).ceil() as u32 + 32;
    let mut radians = true;
    let mut rand_state = rand::RandState::new();
    let mut prev_result = Complex::with_val(precision, 0);

    let colours = RGBValues {
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
        message: (0x5E, 0x35, 0xB1),
    };

    print_stylized_intro(&colours);
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    println!("Goodbye!");
                    break;
                }
                rl.add_history_entry(line.clone())?;

                debug_println(&format!("Processing input: '{}'", line));
                match tokenize(
                    &line,
                    &mut base,
                    &mut precision,
                    &mut digits,
                    &mut radians,
                    &colours,
                    &mut rand_state,
                    &prev_result,
                ) {
                    Ok(tokens) => {
                        match evaluate_tokens(
                            &tokens,
                            base,
                            precision,
                            &mut rand_state,
                            radians,
                            &prev_result,
                        ) {
                            Ok(result) => {
                                let result_vec = num2string(&result, base, digits, &colours);
                                prev_result = result;
                                for coloured_string in result_vec {
                                    print!("{}", coloured_string);
                                }
                                println!();
                            }
                            Err(err) => println!(
                                "{}",
                                err.truecolor(colours.error.0, colours.error.1, colours.error.2)
                            ),
                        }

                        debug_println(&format!("Added to history: {}", line));
                    }
                    Err((msg, pos)) => {
                        if pos == std::usize::MAX {
                            println!(
                                "{}",
                                msg.truecolor(
                                    colours.message.0,
                                    colours.message.1,
                                    colours.message.2
                                )
                            );
                        } else {
                            println!(
                                "  {}{}",
                                " ".repeat(pos),
                                "^".truecolor(colours.carat.0, colours.carat.1, colours.carat.2)
                            );
                            println!(
                                "{}",
                                msg.truecolor(colours.error.0, colours.error.1, colours.error.2)
                            );
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Pressing enter with no input will exit as well.");
                break;
            }
            Err(err) => {
                println!("{:?}", err);
                break;
            }
        }
    }

    Ok(())
}
fn print_stylized_intro(colours: &RGBValues) {
    let ascii_art = r#"
 ____                           _      
|  _ \                         | |     
| |_) | __ _ ___  ___  ___ __ _| | ___ 
|  _ < / _` / __|/ _ \/ __/ _` | |/ __|
| |_) | (_| \__ \  __/ (_| (_| | | (__ 
|____/ \__,_|___/\___|\___\__,_|_|\___|
    "#;

    println!("{}", ascii_art.truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2
    ));

    println!("{}", "Welcome to Basecalc!".truecolor(
        colours.decimal.0,
        colours.decimal.1,
        colours.decimal.2
    ).bold());

    println!("\n{}", "Your gateway to mathematical adventures!".truecolor(
        colours.lone_fraction.0,
        colours.lone_fraction.1,
        colours.lone_fraction.2
    ).italic());

    println!("\n{}", "For help, simply type:".truecolor(
        colours.lone_integer.0,
        colours.lone_integer.1,
        colours.lone_integer.2
    ));

    println!("{}", ":help".truecolor(
        colours.exponent.0,
        colours.exponent.1,
        colours.exponent.2
    ).bold());

    println!("\n{}", "Happy calculating!".truecolor(
        colours.message.0,
        colours.message.1,
        colours.message.2
    ).bold());
}
static OPERATORS: [(&str, char, u8, &str); 27] = [
    // Basic arithmetic
    ("+", '+', 2, "addition"),
    ("-", '-', 2, "subtraction"),
    ("*", '*', 2, "multiplication"),
    ("/", '/', 2, "division"),
    ("^", '^', 2, "exponentiation"),
    ("%", '%', 2, "modulus"),
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
    // Commented out for potential future use
    // ("#gamma", '!', 1, "gamma function"),
    // ("#max", 'M', 2, "maximum"),
    // ("#min", 'm', 2, "minimum"),
];
static CONSTANTS: [(&str, char, &str); 6] = [
    ("@pi", 'p', "Pi"),
    ("@e", 'E', "Euler's number"),
    ("@gamma", 'G', "Euler-Mascheroni constant"),
    ("@rand", 'r', "Random number between 0 and 1"),
    ("@grand", 'g', "Gaussian random number"),
    ("&", '&', "Previous result"),
];
struct RGBValues {
    lone_integer: (u8, u8, u8),
    lone_fraction: (u8, u8, u8),
    real_integer: (u8, u8, u8),
    real_fraction: (u8, u8, u8),
    imaginary_integer: (u8, u8, u8),
    imaginary_fraction: (u8, u8, u8),
    exponent: (u8, u8, u8),
    decimal: (u8, u8, u8),
    sign: (u8, u8, u8),
    tilde: (u8, u8, u8),
    carat: (u8, u8, u8),
    error: (u8, u8, u8),
    brackets: (u8, u8, u8),
    comma: (u8, u8, u8),
    colon: (u8, u8, u8),
    nan: (u8, u8, u8),
    message: (u8, u8, u8),
}
static DEBUG: AtomicBool = AtomicBool::new(false);
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    Addition,
    Multiplication,
    Exponentiation,
    Unary,
    Parenthesis,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Token {
    operator: char,
    operands: u8,
    real_integer: Vec<u8>,
    real_fraction: Vec<u8>,
    imaginary_integer: Vec<u8>,
    imaginary_fraction: Vec<u8>,
    sign: (bool, bool),
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
    fn new() -> Token {
        Token {
            operator: 0 as char,
            operands: 0,
            real_integer: Vec::new(),
            real_fraction: Vec::new(),
            imaginary_integer: Vec::new(),
            imaginary_fraction: Vec::new(),
            sign: (false, false),
        }
    }
}
trait Modulus {
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
fn tokenize(
    input_str: &str,
    base: &mut u8,
    precision: &mut u32,
    digits: &mut usize,
    radians: &mut bool,
    colours: &RGBValues,
    rand_state: &mut rug::rand::RandState,
    prev_result: &Complex,
) -> Result<Vec<Token>, (String, usize)> {
    debug_println(&format!("\nTokenizing: {}", input_str));
    debug_println(&format!(
        "Initial state: base={}, precision={}, digits={}, radians={}",
        base, precision, digits, radians
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
            return parse_command(
                input,
                index + 1,
                base,
                precision,
                digits,
                radians,
                colours,
                rand_state,
                prev_result,
            );
        }
        if input[index] == b'(' {
            if !start && follows_number {
                debug_println(&format!(
                    "Error: Expected operator, found opening parenthesis"
                ));
                return Err((format!("Expected operator!"), index));
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
                return Err((format!("Mismatched parentheses!"), index));
            }
            if !follows_number {
                debug_println(&format!(
                    "Error: Expected number before closing parenthesis"
                ));
                return Err((format!("Expected number!"), index));
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
            match parse_constant(input, index) {
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
            match parse_number(input, base.clone(), index) {
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
                            return Err((msg, pos));
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
            return Err((format!("Invalid operator!"), new_index));
        }
        if token.operands == 1 && follows_number {
            debug_println(&format!("Error: Expected binary operator, found unary"));
            return Err((format!("Expected operator!"), index));
        }
        debug_println(&format!("Parsed operator: {}", token));
        tokens.push(token);
        index = new_index;
        expect_number = true;
        follows_number = false;
    }

    if paren_count != 0 {
        debug_println(&format!("Error: Mismatched parentheses at end of input"));
        return Err((format!("Mismatched parentheses!"), input.len()));
    }

    if tokens.is_empty() {
        debug_println(&format!("Error: Empty expression"));
        return Err((format!("Empty expression"), 0));
    }

    let last_token = tokens.last().unwrap();
    if last_token.operands > 0 && last_token.operator != ')' {
        debug_println(&format!("Error: Incomplete expression at end of input"));
        return Err((format!("Incomplete expression!"), input.len()));
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
fn evaluate_tokens(
    tokens: &[Token],
    base: u8,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    radians: bool,
    prev_result: &Complex,
) -> Result<Complex, String> {
    debug_println("\nEvaluating tokens:");
    let mut output_queue: Vec<Complex> = Vec::new();
    let mut operator_stack: Vec<char> = Vec::new();

    for token in tokens {
        debug_println(&format!("Processing token: {}", token));
        match token.operands {
            0 => {
                // Number or constant
                let mut value = token2num(token, base, precision, rand_state, prev_result);
                debug_println(&format!("Processing number: {}", value));

                // Apply all stacked unary operators
                while let Some(&op) = operator_stack.last() {
                    if get_precedence(op) == Precedence::Unary {
                        debug_println(&format!("Applying stacked unary operator: {}", op));
                        let operator = operator_stack.pop().unwrap();
                        value = apply_unary_operator(operator, value, precision, base, radians)?;
                    } else {
                        break;
                    }
                }

                debug_println(&format!(
                    "Pushed processed number to output queue: {}",
                    value
                ));
                output_queue.push(value);
            }
            1 => {
                // Unary operator or parenthesis
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
                        apply_operator(
                            &mut output_queue,
                            operator_stack.pop().unwrap(),
                            precision,
                            base,
                            radians,
                        )?;
                    }
                    // Apply function if there's one immediately before the parenthesis
                    if let Some(&op) = operator_stack.last() {
                        if get_precedence(op) == Precedence::Unary {
                            apply_operator(
                                &mut output_queue,
                                operator_stack.pop().unwrap(),
                                precision,
                                base,
                                radians,
                            )?;
                        }
                    }
                } else {
                    // Unary operator
                    debug_println(&format!(
                        "Pushed unary operator to stack: {}",
                        token.operator
                    ));
                    operator_stack.push(token.operator);
                }
            }
            2 => {
                // Binary operator
                while let Some(&op) = operator_stack.last() {
                    if op == '(' || get_precedence(token.operator) > get_precedence(op) {
                        break;
                    }
                    apply_operator(
                        &mut output_queue,
                        operator_stack.pop().unwrap(),
                        precision,
                        base,
                        radians,
                    )?;
                }
                operator_stack.push(token.operator);
                debug_println(&format!(
                    "Pushed binary operator to stack: {}",
                    token.operator
                ));
            }
            _ => return Err(format!("Invalid token: {}", token)),
        }
        debug_println(&format!("Output queue: {:?}", output_queue));
        debug_println(&format!("Operator stack: {:?}", operator_stack));
    }

    // Apply remaining operators
    while let Some(op) = operator_stack.pop() {
        if op == '(' {
            return Err("Mismatched parentheses".to_string());
        }
        debug_println(&format!("Applying remaining operator: {}", op));
        apply_operator(&mut output_queue, op, precision, base, radians)?;
    }

    if output_queue.len() != 1 {
        return Err("Invalid expression".to_string());
    }

    Ok(output_queue.pop().unwrap())
}
fn apply_operator(
    output_queue: &mut Vec<Complex>,
    op: char,
    precision: u32,
    base: u8,
    radians: bool,
) -> Result<(), String> {
    debug_println(&format!("Applying operator: {}", op));
    match op {
        '+' | '-' | '*' | '/' | '^' | '%' => apply_binary_operator(output_queue, op)?,
        'n' | 'a' | 'O' | 'o' | 'S' | 'T' | 'c' | 'f' | 'F' | 'i' | 'I' | 'l' | 'L' | 'e' | 'r'
        | 'g' | 's' | 'q' | 't' | 'A' => {
            if let Some(value) = output_queue.pop() {
                let result = apply_unary_operator(op, value, precision, base, radians)?;
                output_queue.push(result);
            } else {
                return Err(format!("Not enough operands for {}", op));
            }
        }
        _ => return Err(format!("Unknown operator: {}", op)),
    }
    Ok(())
}

fn get_precedence(op: char) -> Precedence {
    match op {
        '+' | '-' => Precedence::Addition,
        '*' | '/' | '%' => Precedence::Multiplication,
        '^' => Precedence::Exponentiation,
        'n' | 'a' | 'O' | 'o' | 'S' | 'T' | 'c' | 'f' | 'F' | 'i' | 'I' | 'l' | 'L' | 'e' | 'r'
        | 'g' | 's' | 'q' | 't' | 'A' => Precedence::Unary,
        '(' | ')' => Precedence::Parenthesis,
        _ => Precedence::Addition, // Default to lowest precedence for unknown operators
    }
}
fn apply_unary_operator(
    op: char,
    value: Complex,
    precision: u32,
    base: u8,
    radians: bool,
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
            if radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(precision, rug::float::Constant::Pi)
            }
        }
        'O' => {
            let rad_result = value.acos();
            if radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(precision, rug::float::Constant::Pi)
            }
        }
        'T' => {
            let rad_result = value.atan();
            if radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(precision, rug::float::Constant::Pi)
            }
        }
        'c' => gaussian_ceil(&value),
        'f' => gaussian_floor(&value),
        'F' => fractional_part(&value),
        'i' => Complex::with_val(precision, (value.imag(), 0)),
        'I' => integer_part(&value),
        'l' => value.ln(),
        'L' => value.ln() / Float::with_val(precision, base).ln(),
        'e' => Complex::with_val(precision, (value.real(), 0)),
        'r' => gaussian_round(&value),
        'g' => sign(&value),
        'q' => value.sqrt(),
        's' => {
            if radians {
                value.sin()
            } else {
                let pi = Float::with_val(precision, rug::float::Constant::Pi);
                (value * pi / Float::with_val(precision, 180.0)).sin()
            }
        }
        'o' => {
            if radians {
                value.cos()
            } else {
                let pi = Float::with_val(precision, rug::float::Constant::Pi);
                (value * pi / Float::with_val(precision, 180.0)).cos()
            }
        }
        't' => {
            if radians {
                value.tan()
            } else {
                let pi = Float::with_val(precision, rug::float::Constant::Pi);
                (value * pi / Float::with_val(precision, 180.0)).tan()
            }
        }
        'A' => {
            let rad_result = Complex::with_val(precision, value.imag().clone().atan2(value.real()));
            if radians {
                rad_result
            } else {
                rad_result * 180.0 / Float::with_val(precision, rug::float::Constant::Pi)
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
fn apply_binary_operator(output_queue: &mut Vec<Complex>, op: char) -> Result<(), String> {
    debug_println(&format!("Applying binary operator: {}", op));

    if let (Some(b), Some(a)) = (output_queue.pop(), output_queue.pop()) {
        let result = match op {
            '%' => a.modulus(b),
            '^' => a.pow(&b),
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

fn gaussian_ceil(z: &Complex) -> Complex {
    Complex::with_val(z.prec(), (z.real().clone().ceil(), z.imag().clone().ceil()))
}

fn gaussian_floor(z: &Complex) -> Complex {
    Complex::with_val(
        z.prec(),
        (z.real().clone().floor(), z.imag().clone().floor()),
    )
}

fn fractional_part(z: &Complex) -> Complex {
    z - gaussian_floor(z)
}

fn integer_part(z: &Complex) -> Complex {
    gaussian_floor(z)
}

fn gaussian_round(z: &Complex) -> Complex {
    Complex::with_val(
        z.prec(),
        (z.real().clone().round(), z.imag().clone().round()),
    )
}

fn sign(z: &Complex) -> Complex {
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
fn parse_constant(input: &[u8], index: usize) -> Result<(Token, usize), (String, usize)> {
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
fn parse_number(
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
fn parse_operator(input: &[u8], mut index: usize) -> (Token, usize) {
    let mut token = Token::new();

    if index < input.len() {
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
///
/// # Returns
/// * `Ok(Vec<Token>)` - An empty vector (commands don't produce tokens)
/// * `Err((String, usize))` - A message about the command result and MAX_USIZE
fn parse_command(
    input: &[u8],
    mut index: usize,
    base: &mut u8,
    precision: &mut u32,
    digits: &mut usize,
    radians: &mut bool,
    colours: &RGBValues,
    rand_state: &mut rug::rand::RandState,
    prev_result: &Complex,
) -> Result<Vec<Token>, (String, usize)> {
    let message;
    match &input[index..] {
        s if s.eq_ignore_ascii_case(b"test") => {
            let (passed, total) = run_tests(colours);
            message = format!("{}/{} tests passed.", passed, total);
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
                return Err((format!("Missing base value!"), index));
            }

            let digit = input[index];
            let new_base = if digit.is_ascii_digit() {
                digit - b'0'
            } else if digit.is_ascii_uppercase() {
                digit - b'A' + 10
            } else if digit.is_ascii_lowercase() {
                digit - b'a' + 10
            } else {
                return Err((format!("Invalid base value!"), index));
            };
            if new_base == 1 || new_base > 36 {
                return Err((
                    format!("Base must be between 2 and 36!\nUse ':base 0' for base 36 (Z+1)"),
                    index,
                ));
            }
            *base = if new_base == 0 { 36 } else { new_base };

            let base_char = match *base {
                0..=9 => (*base as u8 + b'0') as char,
                10..=35 => (*base as u8 - 10 + b'A') as char,
                36 => 'Z',
                _ => '?',
            };

            *precision = (*digits as f64 * (*base as f64).log2()).ceil() as u32 + 32;
            message = match get_base_name(*base) {
                Some(name) => {
                    if *base == 36 {
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
                    return Err((format!("Invalid characters after base value!"), index));
                }
                index += 1;
            }
        }
        s if s.len() >= 6 && s[..6].eq_ignore_ascii_case(b"digits") => {
            let token = Token::new();
            let value;
            let new_index;
            match parse_number(input, base.clone(), index + 6) {
                Ok((token, x)) => {
                    new_index = x;
                    if token.real_fraction.len() > 0
                        || token.imaginary_integer.len() > 0
                        || token.imaginary_fraction.len() > 0
                        || token.sign.0
                    {
                        return Err((format!("Precision must be a positive real integer!"), index));
                    }

                    value = token2num(&token, *base, *precision, rand_state, prev_result)
                        .real()
                        .clone()
                        .round()
                        .to_f64() as usize;
                    if value == 0 {
                        return Err((format!("Precision must be a positive real integer!"), index));
                    }
                    message = format!(
                        "Precision set to {} digits.",
                        format_int(value, *base as usize)
                    );
                }
                Err((msg, pos)) => {
                    return Err((msg, pos));
                }
            }
            index = new_index;

            // Check if there's anything after the number
            if index < input.len() {
                for i in index..input.len() {
                    if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                        return Err((format!("Invalid characters after digits value!"), i));
                    }
                }
            }
            *digits = value;
            *precision = (*digits as f64 * (*base as f64).log2()).ceil() as u32 + 32;
            if token.imaginary_integer.len() > 0 || token.imaginary_fraction.len() > 0 {
                return Err((format!("Precision must be a real integer!"), index));
            }
        }
        s if s.len() >= 7 && s[..7].eq_ignore_ascii_case(b"degrees") => {
            // Check if there's anything after the command
            for i in index + 7..input.len() {
                if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                    return Err((format!("Invalid characters after command!"), i));
                }
            }
            *radians = false;
            message = format!("Angle units set to degrees.");
        }
        s if s.len() >= 7 && s[..7].eq_ignore_ascii_case(b"radians") => {
            // Check if there's anything after the command
            for i in index + 7..input.len() {
                if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                    return Err((format!("Invalid characters after command!"), i));
                }
            }
            *radians = true;
            message = format!("Angle units set to radians.");
        }
        s if s.eq_ignore_ascii_case(b"help") => {
            let help_text = get_help_text(
                colours,
                *base,
                *precision,
                *digits,
                *radians,
                rand_state,
                prev_result,
            );
            for line in help_text {
                print!("{}", line);
            }
            message = "\n".to_string();
        }
        s if s.len() >= 5 && s[..5].eq_ignore_ascii_case(b"debug") => {
            // Toggle debug mode
            let new_state = !DEBUG.load(Ordering::Relaxed);
            DEBUG.store(new_state, Ordering::Relaxed);
            message = format!("Debug {}", if new_state { "enabled" } else { "disabled" });
        }
        _ => return Err((format!("Unknown command!"), index)),
    };

    Err((message, std::usize::MAX))
}
fn get_help_text(
    colours: &RGBValues,
    base: u8,
    precision: u32,
    digits: usize,
    radians: bool,
    rand_state: &mut rug::rand::RandState,
    prev_result: &Complex,
) -> Vec<ColoredString> {
    let mut help_text: Vec<ColoredString> = Vec::new();

    // Geeky Intro
    help_text.push("Welcome to basecalc!\n".truecolor(
        colours.decimal.0,
        colours.decimal.1,
        colours.decimal.2,
    ));
    help_text.push("
Greetings, intrepid mathematical explorer!  This isn't just any ordinary number-crunching gizmo - it's your towel in the cosmos!

Whether you're calculating the odds of successfully navigating an asteroid field, determining the exact amount of Pangalactic Gargleblasters needed for a party of trans-dimensional beings, or just trying to split the bill at the Restaurant at the End of the Universe, basecalc has got you covered!

Remember, DON'T PANIC! With basecalc, you're always just a few keystrokes away from mathematical enlightenment. So grab your towel, keep your wits about you, and prepare to compute where no one has computed before!
".normal());

    // Commands
    help_text.push("\nCommands:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    let commands = [
        (
            ":base ",
            "<digit>  ",
            "Set the number base (2 to Z+1), use 0 for Z+1",
        ),
        (":digits ", "<value>", "Set the number of digits to display"),
        (":radians       ", "", "Set angle units to radians"),
        (":degrees       ", "", "Set angle units to degrees"),
        (":help          ", "", "Display this help message"),
        (":debug         ", "", "Toggle debug mode"),
        (":test          ", "", "Run internal tests"),
    ];

    for (cmd, alt, desc) in commands.iter() {
        help_text.push(format!("  {}", cmd).truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2,
        ));
        help_text.push(alt.truecolor(colours.nan.0, colours.nan.1, colours.nan.2));
        help_text.push(format!(" - {}\n", desc).truecolor(
            colours.lone_fraction.0,
            colours.lone_fraction.1,
            colours.lone_fraction.2,
        ));
    }

    // Constants
    help_text.push("\nConstants:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    for &(name, symbol, description) in CONSTANTS.iter() {
        let token = Token {
            operator: symbol,
            ..Token::new()
        };
        let value = token2num(&token, base, precision, rand_state, prev_result);
        let value_string = num2string(&value, base, digits, colours);

        help_text.push(format!("  {:<7}", name).truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2,
        ));
        help_text.push(format!("- {} ", description).truecolor(
            colours.lone_fraction.0,
            colours.lone_fraction.1,
            colours.lone_fraction.2,
        ));
        for part in value_string {
            help_text.push(part);
        }
        help_text.push("\n".truecolor(colours.brackets.0, colours.brackets.1, colours.brackets.2));
    }

    // Operators and Functions
    help_text.push("\nUnary Operators:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    for &(name, _, operands, description) in OPERATORS.iter() {
        if operands == 1 && name != "(" && name != ")" {
            help_text.push(format!("  {:<8}", name).truecolor(
                colours.lone_integer.0,
                colours.lone_integer.1,
                colours.lone_integer.2,
            ));
            let capitalized_description = description[0..1].to_uppercase() + &description[1..];
            help_text.push(format!("- {}\n", capitalized_description).truecolor(
                colours.lone_fraction.0,
                colours.lone_fraction.1,
                colours.lone_fraction.2,
            ));
        }
    }

    help_text.push("\nBinary Operators:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    for &(name, _, operands, description) in OPERATORS.iter() {
        if operands == 2 {
            help_text.push(format!("  {:<7}", name).truecolor(
                colours.lone_integer.0,
                colours.lone_integer.1,
                colours.lone_integer.2,
            ));
            let capitalized_description = description[0..1].to_uppercase() + &description[1..];
            help_text.push(format!("- {}\n", capitalized_description).truecolor(
                colours.lone_fraction.0,
                colours.lone_fraction.1,
                colours.lone_fraction.2,
            ));
        }
    }

    // Grouping
    help_text.push("\nGrouping:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    help_text.push("  ( )   ".truecolor(
        colours.lone_integer.0,
        colours.lone_integer.1,
        colours.lone_integer.2,
    ));
    help_text.push("- Parentheses for grouping expressions\n".truecolor(
        colours.lone_fraction.0,
        colours.lone_fraction.1,
        colours.lone_fraction.2,
    ));

    // Usage
    help_text.push("\nUsage:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    let usage_points = [
        "Enter expressions using operators, functions, and constants.",
        "Use [] for complex numbers, e.g., [3, 4] for 3 + 4i.",
        "Use parentheses () to group expressions.",
        "Spaces are optional in all cases, but can be used for readability.",
        "Type a command or expression and press 'Enter' to evaluate.",
        "Case insensitive parsing of all entries.",
    ];
    for point in usage_points.iter() {
        help_text.push("- ".truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2,
        ));
        help_text.push(format!("{}\n", point).truecolor(
            colours.lone_fraction.0,
            colours.lone_fraction.1,
            colours.lone_fraction.2,
        ));
    }

    // Examples
    help_text.push("\nExamples:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    let examples = [
        ("2 + 3 * 4", "Basic arithmetic"),
        ("#sin(@pi/4)", "Function with constant"),
        ("[3, 4] * [1, -1]", "Complex number multiplication"),
        ("#sqrt-1", "Imaginary number"),
        ("#log(100)/2", "Logarithm and division"),
        (":base C", "Set base to Dozenal"),
        (":digits 10", "Set display digits to 10"),
        ("5^ -25 * [-3.24,-4.1b]", "Exponentiation with complex numbers"),
        (":base A", "Set base to Decimal"),
        ("9+1", "Most humans are used to this"),
        ("#cos(@pi/3)", "Cosine function"),
        ("#tan #sin(@pi/4)", "Nested trigonometric functions"),
        ("&^2", "Square the previous result"),
        ("#ceil 3.7 + #floor(2.1)", "Ceiling and floor functions"),
        ("@e^#ln2", "Natural exponent and logarithm"),
        ("#abs[-3,4]", "Absolute value of a complex number"),
        ("[1,2]/[1,-2]", "Complex division"),
        (":degrees", "Set angle units to degrees"),
        ("#asin(0.5)", "Arcsine function in degrees"),
        ("@rand", "Generate a random number"),
        ("@grand", "Generate a Gaussian random number"),
        ("#frac(5.7) + #int(3.2)", "Fractional and integer parts"),
        ("17%5", "Modulus operation"),
        (":baseg", "Set base to Hexadecimal"),
        (":base 2", "Set base to Binary"),
        (":base A", "Return to Decimal (lame)"),
    ];

    let mut local_base = base;
    let mut local_precision = precision;
    let mut local_digits = digits;
    let mut local_radians = radians;
    let mut local_prev_result = Complex::with_val(precision, 0);

    for (example, desc) in examples.iter() {
        help_text.push(format!("- {}\n", desc).truecolor(
            colours.lone_fraction.0,
            colours.lone_fraction.1,
            colours.lone_fraction.2,
        ));
        help_text.push(format!("  {}\n", example).truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2,
        ));

        if example.starts_with(':') {
            // Handle commands
            match parse_command(
                example.as_bytes(),
                1,
                &mut local_base,
                &mut local_precision,
                &mut local_digits,
                &mut local_radians,
                colours,
                rand_state,
                &local_prev_result,
            ) {
                Err((msg, _)) => {
                    help_text.push(format!("  {}\n", msg).truecolor(
                        colours.message.0,
                        colours.message.1,
                        colours.message.2,
                    ));
                }
                _ => {}
            }
        } else {
            // Handle expressions
            match tokenize(
                example,
                &mut local_base,
                &mut local_precision,
                &mut local_digits,
                &mut local_radians,
                colours,
                rand_state,
                &local_prev_result,
            ) {
                Ok(tokens) => {
                    match evaluate_tokens(
                        &tokens,
                        local_base,
                        local_precision,
                        rand_state,
                        local_radians,
                        &local_prev_result,
                    ) {
                        Ok(result) => {
                            help_text.push("  ".normal());
                            let result_string =
                                num2string(&result, local_base, local_digits, colours);
                            for part in result_string {
                                help_text.push(part);
                            }
                            help_text.push("\n".normal());
                            local_prev_result = result; // Update local_prev_result for & usage
                        }
                        Err(err) => {
                            help_text.push(format!("  Error: {}\n", err).truecolor(
                                colours.error.0,
                                colours.error.1,
                                colours.error.2,
                            ));
                        }
                    }
                }
                Err((msg, _)) => {
                    help_text.push(format!("  Error: {}\n", msg).truecolor(
                        colours.error.0,
                        colours.error.1,
                        colours.error.2,
                    ));
                }
            }
        }
        help_text.push("\n".normal());
    }

    // Tips
    help_text.push("\nTips:\n".truecolor(
        colours.brackets.0,
        colours.brackets.1,
        colours.brackets.2,
    ));
    let tips = [
        "Use the '&' symbol to refer to the previous result in calculations.",
        "Toggle debug mode with ':debug' to see detailed calculation steps.",
        "Run ':test' to verify calculator functionality.",
    ];
    for tip in tips.iter() {
        help_text.push("- ".truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2,
        ));
        help_text.push(format!("{}\n", tip).truecolor(
            colours.lone_fraction.0,
            colours.lone_fraction.1,
            colours.lone_fraction.2,
        ));
    }

    // Current Settings
help_text.push("\nCurrent Settings:\n".truecolor(
    colours.brackets.0,
    colours.brackets.1,
    colours.brackets.2,
));

// Base
help_text.push("Base: ".truecolor(
    colours.lone_integer.0,
    colours.lone_integer.1,
    colours.lone_integer.2,
));
help_text.push(format!("{}\n", base).truecolor(
    colours.lone_fraction.0,
    colours.lone_fraction.1,
    colours.lone_fraction.2,
));

// Precision
help_text.push("Precision: ".truecolor(
    colours.lone_integer.0,
    colours.lone_integer.1,
    colours.lone_integer.2,
));
help_text.push(format!("{}\n", precision).truecolor(
    colours.lone_fraction.0,
    colours.lone_fraction.1,
    colours.lone_fraction.2,
));

// Display Digits
help_text.push("Display Digits: ".truecolor(
    colours.lone_integer.0,
    colours.lone_integer.1,
    colours.lone_integer.2,
));
help_text.push(format!("{}\n", digits).truecolor(
    colours.lone_fraction.0,
    colours.lone_fraction.1,
    colours.lone_fraction.2,
));

// Angle Units
help_text.push("Angle Units: ".truecolor(
    colours.lone_integer.0,
    colours.lone_integer.1,
    colours.lone_integer.2,
));
help_text.push(format!("{}\n", if radians { "Radians" } else { "Degrees" }).truecolor(
    colours.lone_fraction.0,
    colours.lone_fraction.1,
    colours.lone_fraction.2,
));

    help_text.push(
        "\nFor more detailed information, comments, questions or why 42, contact nick spiker.".normal(),
    );

    help_text
}
fn generate_random(precision: u32, rand_state: &mut rug::rand::RandState) -> Complex {
    let real = Float::with_val(precision, Float::random_cont(rand_state));
    Complex::with_val(precision, (real, 0))
}
fn gaussian_complex_random(precision: u32, rand_state: &mut rug::rand::RandState) -> Complex {
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
/// * `base` - The current number base
/// * `precision` - The precision for the resulting number
///
/// # Returns
/// * `Complex` - The complex number representation of the token
fn token2num(
    token: &Token,
    base: u8,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    prev_result: &Complex,
) -> Complex {
    match token.operator {
        'E' => Complex::with_val(precision, Float::with_val(precision, 1).exp()),
        'G' => Complex::with_val(precision, rug::float::Constant::Euler),
        'p' => Complex::with_val(precision, rug::float::Constant::Pi),
        'r' => generate_random(precision, rand_state),
        'g' => gaussian_complex_random(precision, rand_state),
        '&' => prev_result.clone(),
        _ => {
            let mut real_int = Float::with_val(precision, 0);
            for &digit in &token.real_integer {
                real_int *= base;
                real_int += digit;
            }
            let mut real_frac = Float::with_val(precision, 0);
            for &digit in token.real_fraction.iter().rev() {
                real_frac += digit as f64;
                real_frac /= base as f64;
            }

            let mut imag_int = Float::with_val(precision, 0);
            for &digit in &token.imaginary_integer {
                imag_int *= base;
                imag_int += digit;
            }
            let mut imag_frac = Float::with_val(precision, 0);
            for &digit in token.imaginary_fraction.iter().rev() {
                imag_frac += digit as f64;
                imag_frac /= base as f64;
            }

            let mut real = Float::with_val(precision, &real_int + &real_frac);
            let mut imaginary = Float::with_val(precision, &imag_int + &imag_frac);

            if token.sign.0 {
                real = -real;
            }
            if token.sign.1 {
                imaginary = -imaginary;
            }

            Complex::with_val(precision, (real, imaginary))
        }
    }
}
/// Converts a complex number to a vector of coloured strings for display
///
/// # Arguments
/// * `num` - The complex number to convert
/// * `base` - The current number base
/// * `digits` - The number of digits to display
/// * `colours` - The colour scheme for output formatting
///
/// # Returns
/// * `Vec<ColoredString>` - A vector of coloured strings representing the number
fn num2string(num: &Complex, base: u8, digits: usize, colours: &RGBValues) -> Vec<ColoredString> {
    let mut result = Vec::new();

    if num.real().is_nan()
        || num.imag().is_nan()
        || num.real().is_infinite()
        || num.imag().is_infinite()
    {
        result.push("NaN".truecolor(colours.nan.0, colours.nan.1, colours.nan.2));
        return result;
    }

    if num.imag().is_zero() {
        result.push(" ".normal());
        result.extend(format_part(num.real(), base, digits, colours, true, true));
    } else {
        result.push("[".truecolor(colours.brackets.0, colours.brackets.1, colours.brackets.2));
        result.extend(format_part(num.real(), base, digits, colours, true, false));
        result.push(" ,".truecolor(colours.comma.0, colours.comma.1, colours.comma.2));
        result.extend(format_part(num.imag(), base, digits, colours, false, false));
        result.push(" ]".truecolor(colours.brackets.0, colours.brackets.1, colours.brackets.2));
    }

    result
}
/// Formats a part of a complex number (real or imaginary) as a vector of coloured strings
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
/// * `Vec<ColoredString>` - A vector of coloured strings representing the formatted number
fn format_part(
    num: &rug::Float,
    base: u8,
    num_digits: usize,
    colours: &RGBValues,
    is_real: bool,
    is_lone: bool,
) -> Vec<ColoredString> {
    let mut result = Vec::new();

    if num.is_zero() {
        result.push(" ".normal());
        result.push("0".truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2,
        ));
        result.push(".".truecolor(colours.decimal.0, colours.decimal.1, colours.decimal.2));
        return result;
    }
    if num.is_nan() || num.is_infinite() {
        result.push("NaN".truecolor(colours.nan.0, colours.nan.1, colours.nan.2));
        return result;
    }

    let is_positive = num.is_sign_positive();
    if is_positive {
        result.push(" ".normal());
    } else {
        result.push("-".truecolor(colours.sign.0, colours.sign.1, colours.sign.2));
    }

    let mut num_abs = num.clone().abs();
    let mut decimal_place = (num_abs.clone().log2() / (Float::with_val(num.prec(), base)).log2())
        .floor()
        .to_f64() as isize;
    num_abs = num_abs / (Float::with_val(num.prec(), base)).pow(decimal_place);
    num_abs += (Float::with_val(num.prec(), base)).pow(-(num_digits as isize - 1)) / 2;
    if num_abs > base {
        num_abs = num.clone().abs();
        decimal_place += 1;
        num_abs = num_abs / (Float::with_val(num.prec(), base)).pow(decimal_place);
        num_abs += (Float::with_val(num.prec(), base)).pow(-(num_digits as isize - 1)) / 2;
    }

    let mut integer_part = String::new();
    let mut decimal = false;
    let mut place = 0;
    let mut offset = place as isize - decimal_place;
    while offset <= 0 && place < num_digits {
        place += 1;
        let digit: u8 = num_abs.clone().floor().cast();
        num_abs = num_abs - digit;
        num_abs *= base;
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
    while offset > 0 && place < num_digits {
        place += 1;
        let digit: u8 = num_abs.clone().floor().cast();
        num_abs = num_abs - digit;
        num_abs *= base;
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
    let (int_colour, frac_colour) = if is_lone {
        (colours.lone_integer, colours.lone_fraction)
    } else if is_real {
        (colours.real_integer, colours.real_fraction)
    } else {
        (colours.imaginary_integer, colours.imaginary_fraction)
    };
    let prec = num_abs.prec();
    let tilde =
        (num_abs * Float::with_val(prec, 2) - Float::with_val(prec, base)).abs() > 2f64.pow(-16);
    if decimal {
        if integer_part.is_empty() {
            result.push("0".truecolor(int_colour.0, int_colour.1, int_colour.2));
        } else {
            result.push(integer_part.truecolor(int_colour.0, int_colour.1, int_colour.2));
        }
        result.push(".".truecolor(colours.decimal.0, colours.decimal.1, colours.decimal.2));
        result.push(trim_zeros(fractional_part).truecolor(
            frac_colour.0,
            frac_colour.1,
            frac_colour.2,
        ));
        if tilde {
            result.push("~".truecolor(colours.tilde.0, colours.tilde.1, colours.tilde.2));
        } else {
            result.push(" ".normal());
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
            result.push(number.truecolor(frac_colour.0, frac_colour.1, frac_colour.2));
            if tilde {
                result.push("~".truecolor(colours.tilde.0, colours.tilde.1, colours.tilde.2));
            } else {
                result.push(" ".normal());
            }
            result.push(" :".truecolor(colours.colon.0, colours.colon.1, colours.colon.2));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, base as usize));
                result.push(exponent.truecolor(
                    colours.exponent.0,
                    colours.exponent.1,
                    colours.exponent.2,
                ));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, base as usize));
                result.push(exponent.truecolor(
                    colours.exponent.0,
                    colours.exponent.1,
                    colours.exponent.2,
                ));
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
            result.push(number.truecolor(int_colour.0, int_colour.1, int_colour.2));
            if tilde {
                result.push("~".truecolor(colours.tilde.0, colours.tilde.1, colours.tilde.2));
            } else {
                result.push(" ".normal());
            }
            result.push(" :".truecolor(colours.colon.0, colours.colon.1, colours.colon.2));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, base as usize));
                result.push(exponent.truecolor(
                    colours.exponent.0,
                    colours.exponent.1,
                    colours.exponent.2,
                ));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, base as usize));
                result.push(exponent.truecolor(
                    colours.exponent.0,
                    colours.exponent.1,
                    colours.exponent.2,
                ));
            }
        }
    }
    result
}
fn trim_zeros(mut number: String) -> String {
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
fn format_int(mut num: usize, base: usize) -> String {
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
fn get_base_name(base: u8) -> Option<&'static str> {
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
fn debug_println(msg: &str) {
    if DEBUG.load(Ordering::Relaxed) {
        println!("{}", msg);
    }
}
fn run_tests(colours: &RGBValues) -> (usize, usize) {
    let mut base = 10;
    let mut digits = 12;
    let mut precision = (digits as f64 * (base as f64).log2()).ceil() as u32 + 32;
    let mut radians = true;
    let mut rand_state = rand::RandState::new();

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
    ];
    let mut passed = 0;
    let total = tests.len();
    let mut prev_value = Complex::with_val(precision, 0);
    for (input, expected) in tests {
        println!("> {}", input);

        let (coloured_result, result) = match tokenize(
            input,
            &mut base,
            &mut precision,
            &mut digits,
            &mut radians,
            colours,
            &mut rand_state,
            &prev_value,
        ) {
            Ok(tokens) => {
                match evaluate_tokens(
                    &tokens,
                    base,
                    precision,
                    &mut rand_state,
                    radians,
                    &prev_value,
                ) {
                    Ok(eval_value) => {
                        let coloured_vec = num2string(&eval_value, base, digits, &colours);
                        prev_value = eval_value;
                        (coloured_vec.clone(), coloured_vec_to_string(&coloured_vec))
                    }
                    Err(err) => (vec![err.red()], err),
                }
            }
            Err((msg, _)) => (
                vec![msg.truecolor(colours.message.0, colours.message.1, colours.message.2)],
                msg,
            ),
        };

        for coloured_string in &coloured_result {
            print!("{}", coloured_string);
        }
        println!();

        if result == expected {
            println!("{}", "Pass!".green());
            passed += 1;
        } else {
            println!("{}", "fail!".red());
            println!("Sposta: '{}'", expected);
            println!("Gots  : '{}'", result);
        }

        println!();
    }
    (passed, total)
}
fn coloured_vec_to_string(coloured_vec: &Vec<ColoredString>) -> String {
    let mut result = String::new();
    for coloured_string in coloured_vec {
        for c in coloured_string.chars() {
            if c.is_ascii() {
                result.push(c);
            }
        }
    }
    result.trim_end().to_owned()
}
