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

    let colours = RGBValues {
        lone_integer: (0xB4, 0xB4, 0xB4),       // Light gray
        lone_fraction: (0x8C, 0x64, 0x8C),      // Muted purple
        real_integer: (0xB4, 0x8C, 0x8C),       // Muted red
        real_fraction: (0x8C, 0x3C, 0x64),      // Dark red
        imaginary_integer: (0x8C, 0x8C, 0xB4),  // Muted blue
        imaginary_fraction: (0x64, 0x3C, 0x8C), // Dark purple
        exponent: (0xDC, 0xF0, 0x32),           // Bright yellow
        decimal: (0xFF, 0xFF, 0xFF),            // White
        sign: (0xFF, 0xFF, 0xFF),               // White
        tilde: (0x50, 0x8C, 0x78),              // Muted teal
        carat: (0xFF, 0x14, 0x00),              // Bright red
        error: (0xDC, 0x64, 0x5A),              // Soft red
        brackets: (0xB4, 0xBE, 0x3C),           // Olive green
        comma: (0xFF, 0xBE, 0x00),              // Orange
        colon: (0x28, 0x50, 0x14),              // Dark green
        nan: (0xC8, 0x64, 0xC8),                // Bright purple
        message: (0x78, 0xB4, 0x78),            // Soft green
    };

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
                ) {
                    Ok(tokens) => {
                        debug_println(&format!("Tokens: {:?}", tokens));
                        match evaluate_tokens(&tokens, base, precision, &mut rand_state, radians) {
                            Ok(result) => {
                                let result_vec = num2string(&result, base, digits, &colours);
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
    Lowest,
    Addition,
    Multiplication,
    Exponentiation,
    UnaryNegation,
    Function,
    Constant,
    Highest,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Token {
    operator: char,
    operands: u8,
    real_integer: Vec<u8>,
    real_fraction: Vec<u8>,
    imaginary_integer: Vec<u8>,
    imaginary_fraction: Vec<u8>,
    sign: (bool, bool),
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

fn tokenize(
    input_str: &str,
    base: &mut u8,
    precision: &mut u32,
    digits: &mut usize,
    radians: &mut bool,
    colours: &RGBValues,
) -> Result<Vec<Token>, (String, usize)> {
    debug_println(&format!("Tokenizing: {}", input_str));
    let input = input_str.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut paren_count = 0;

    while index < input.len() {
        if input[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if input[index] == b':' {
            return parse_command(input, index + 1, base, precision, digits, radians, colours);
        }

        match input[index] {
            b'(' => {
                tokens.push(Token {
                    operator: '(',
                    operands: 0,
                    ..Token::new()
                });
                paren_count += 1;
                index += 1;
            }
            b')' => {
                if paren_count == 0 {
                    return Err((format!("Mismatched parentheses!"), index));
                }
                tokens.push(Token {
                    operator: ')',
                    operands: 0,
                    ..Token::new()
                });
                paren_count -= 1;
                index += 1;
            }
            b'#' => {
                let (token, new_index) = parse_operator(input, index)?;
                tokens.push(token);
                index = new_index;
            }
            b'@' => {
                let (token, new_index) = parse_constant(input, index)?;
                tokens.push(token);
                index = new_index;
            }
            b'-' => {
                let is_unary = tokens.is_empty()
                    || matches!(
                        tokens.last().unwrap().operator,
                        '(' | '+' | '-' | '*' | '/' | '^' | '%' | '#' | 'n'
                    );
                if is_unary {
                    tokens.push(Token {
                        operator: 'n', // 'n' for unary negation
                        operands: 1,
                        ..Token::new()
                    });
                } else {
                    tokens.push(Token {
                        operator: '-',
                        operands: 2,
                        ..Token::new()
                    });
                }
                index += 1;
            }
            _ => {
                if input[index].is_ascii_digit() || input[index] == b'.' || input[index] == b'[' {
                    let mut number_token = Token::new();
                    let new_index = parse_number(input, &mut number_token, *base, index)?;
                    tokens.push(number_token);
                    index = new_index;
                } else {
                    let (token, new_index) = parse_operator(input, index)?;
                    tokens.push(token);
                    index = new_index;
                }
            }
        }
    }

    if paren_count != 0 {
        return Err((format!("Mismatched parentheses!"), input.len()));
    }

    if tokens.is_empty() {
        return Err((format!("Empty expression"), 0));
    }

    // Check for incomplete expressions
    let last_token = tokens.last().unwrap();
    if last_token.operator != '\0' && last_token.operator != ')' && last_token.operands > 0 {
        return Err((format!("Incomplete expression!"), input.len()));
    }

    for token in &tokens {
        debug_println(&format!("Token: {:?}", token));
    }

    Ok(tokens)
}
fn evaluate_tokens(
    tokens: &[Token],
    base: u8,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    radians: bool,
) -> Result<Complex, String> {
    debug_println("Evaluating tokens:");
    let mut output_queue: Vec<Complex> = Vec::new();
    let mut operator_stack: Vec<char> = Vec::new();

    for token in tokens {
        debug_println(&format!("Processing token: {:?}", token));
        if token.operator == '\0' {
            let value = token2num(token, base, precision);
            debug_println(&format!("  Pushed number: {}", value));
            output_queue.push(value);
        } else if token.operator == '(' {
            operator_stack.push('(');
        } else if token.operator == ')' {
            while let Some(&op) = operator_stack.last() {
                if op == '(' {
                    break;
                }
                apply_operator(
                    &mut output_queue,
                    operator_stack.pop().unwrap(),
                    precision,
                    rand_state,
                    base,
                    radians,
                )?;
            }
            if operator_stack.pop() != Some('(') {
                return Err("Mismatched parentheses".to_string());
            }
        } else {
            while !operator_stack.is_empty() {
                let top_op = *operator_stack.last().unwrap();
                if top_op == '(' {
                    break;
                }
                if (get_precedence(top_op) > get_precedence(token.operator))
                    || (get_precedence(top_op) == get_precedence(token.operator)
                        && token.operator != '^'
                        && token.operator != 'n')
                {
                    apply_operator(
                        &mut output_queue,
                        operator_stack.pop().unwrap(),
                        precision,
                        rand_state,
                        base,
                        radians,
                    )?;
                } else {
                    break;
                }
            }
            operator_stack.push(token.operator);
            debug_println(&format!("Output queue: {:?}", output_queue));
            debug_println(&format!("Operator stack: {:?}", operator_stack));
        }
    }

    while let Some(op) = operator_stack.pop() {
        apply_operator(&mut output_queue, op, precision, rand_state, base, radians)?;
    }

    if output_queue.len() != 1 {
        return Err("Invalid expression!".to_string());
    }

    Ok(output_queue.pop().unwrap())
}
fn apply_operator(
    output_queue: &mut Vec<Complex>,
    op: char,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    base: u8,
    radians: bool,
) -> Result<(), String> {
    debug_println(&format!("Applying operator: {}", op));
    match op {
        'E' => output_queue.push(Complex::with_val(
            precision,
            Float::with_val(precision, 1).exp(),
        )),
        'G' => output_queue.push(Complex::with_val(precision, rug::float::Constant::Euler)),
        'p' => output_queue.push(Complex::with_val(precision, rug::float::Constant::Pi)),
        'r' => output_queue.push(generate_random(precision, rand_state)),
        'g' => output_queue.push(gaussian_complex_random(precision, rand_state)),
        'n' => {
            if let Some(operand) = output_queue.pop() {
                debug_println(&format!(
                    "Result after unary negation: {:?}",
                    -operand.clone()
                ));
                output_queue.push(-operand);
            } else {
                return Err("Not enough operands for unary negation".to_string());
            }
        }
        'a' | 'C' | 'S' | 'T' | 'c' | 'i' | 'l' | 'L' | 'e' | 's' | 'q' | 't' => {
            if let Some(operand) = output_queue.pop() {
                let result = if radians {
                    match op {
                        'a' => operand.abs(),
                        'C' => operand.acos(),
                        'S' => operand.asin(),
                        'T' => operand.atan(),
                        'c' => operand.cos(),
                        'i' => Complex::with_val(precision, (operand.imag(), 0)),
                        'l' => operand.ln(),
                        'L' => operand.ln() / Float::with_val(precision, base).ln(),
                        'e' => Complex::with_val(precision, (operand.real(), 0)),
                        's' => operand.sin(),
                        'q' => operand.sqrt(),
                        't' => operand.tan(),
                        _ => unreachable!(),
                    }
                } else {
                    let pi = Float::with_val(precision, rug::float::Constant::Pi);
                    match op {
                        'a' => operand.abs(),
                        'C' => {
                            let rad_result = operand.acos();
                            rad_result * 180.0 / pi
                        }
                        'S' => {
                            let rad_result = operand.asin();
                            rad_result * 180.0 / pi
                        }
                        'T' => {
                            let rad_result = operand.atan();
                            rad_result * 180.0 / pi
                        }
                        'c' => {
                            let rad_operand: Complex = operand * pi / 180.0;
                            rad_operand.cos()
                        }
                        'i' => Complex::with_val(precision, (operand.imag(), 0)),
                        'l' => operand.ln(),
                        'L' => operand.ln() / Float::with_val(precision, base).ln(),
                        'e' => Complex::with_val(precision, (operand.real(), 0)),
                        's' => {
                            let rad_operand: Complex = operand * pi / 180.0;
                            rad_operand.sin()
                        }
                        'q' => operand.sqrt(),
                        't' => {
                            let rad_operand: Complex = operand * pi / 180.0;
                            rad_operand.tan()
                        }
                        _ => unreachable!(),
                    }
                };
                debug_println(&format!("Result after operation: {:?}", result));
                output_queue.push(result);
            } else {
                return Err(format!("Not enough operands for operator {}", op));
            }
        }
        _ => {
            if let (Some(b), Some(a)) = (output_queue.pop(), output_queue.pop()) {
                let result = match op {
                    '%' => a.modulus(b),
                    '^' => a.pow(&b),
                    '*' => a * b,
                    '+' => a + b,
                    '-' => a - b,
                    '/' => a / b,
                    _ => return Err(format!("Unknown operator: {}", op)),
                };
                debug_println(&format!("Result after operation: {:?}", result));
                output_queue.push(result);
            } else {
                return Err(format!("Not enough operands for operator {}", op));
            }
        }
    }
    Ok(())
}
fn parse_constant(input: &[u8], index: usize) -> Result<(Token, usize), (String, usize)> {
    let constants = [
        ("@e", 'E'),     // e (Euler's number)
        ("@gamma", 'G'), // γ Euler-Mascheroni
        ("@grand", 'g'), // Gaussian random
        ("@pi", 'p'),    // Pi
        ("@rand", 'r'),  // Random
    ];

    for &(name, op) in &constants {
        if input[index..].starts_with(name.as_bytes()) {
            return Ok((
                Token {
                    operator: op,
                    operands: 0,
                    ..Token::new()
                },
                index + name.len(),
            ));
        }
    }

    Err((format!("Invalid constant!"), index))
}
fn parse_number(
    input: &[u8],
    token: &mut Token,
    base: u8,
    mut index: usize,
) -> Result<usize, (String, usize)> {
    let mut complex = false;
    let mut imaginary = false;
    let mut integer = true;
    let mut expect_sign = false;

    while index < input.len() {
        let c = input[index];

        if c == b' ' || c == b'_' || c == b'\t' {
            index += 1;
            continue;
        }

        if c == b'[' {
            if !token.real_integer.is_empty() || !token.real_fraction.is_empty() {
                return Err((format!("Unexpected '['"), index));
            }
            complex = true;
            expect_sign = true;
            index += 1;
            continue;
        }

        if expect_sign {
            if c == b'-' {
                if imaginary {
                    token.sign.1 = true;
                } else {
                    token.sign.0 = true;
                }
                index += 1;
            }
            expect_sign = false;
            continue;
        }

        if c == b',' {
            if !complex {
                return Err((format!("Unexpected ','"), index));
            }
            imaginary = true;
            integer = true;
            expect_sign = true;
            index += 1;
            continue;
        }

        if c == b']' {
            if !complex {
                return Err((format!("Unexpected ']'"), index));
            }
            return Ok(index + 1);
        }

        if c == b'.' {
            if !integer {
                return Err((format!("Multiple decimal points"), index));
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
            return Ok(index); // End of number
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
        return Err((format!("Unclosed complex number"), index));
    }

    Ok(index)
}
fn parse_operator(input: &[u8], mut index: usize) -> Result<(Token, usize), (String, usize)> {
    let operators = [
        // ("operator", 'operator symbol', operands)
        ("#abs", 'a', 1),  // Absolute value
        ("#acos", 'C', 1), // Inverse cosine
        ("#asin", 'S', 1), // Inverse sine
        ("#atan", 'T', 1), // Inverse tangent
        ("#cos", 'c', 1),  // Cosine
        ("#im", 'i', 1),   // Imaginary10
        ("#ln", 'l', 1),   // Natural logarithm
        ("#log", 'L', 1),  // Base logarithm
        ("#re", 'e', 1),   // Real
        ("#sin", 's', 1),  // Sine
        ("#sqrt", 'q', 1), // Square root
        ("#tan", 't', 1),  // Tangent
        ("%", '%', 2),     // Modulus
        ("*", '*', 2),     // Multiplication
        ("+", '+', 2),     // Addition
        ("-", '-', 2),     // Subtraction
        ("/", '/', 2),     // Division
        ("^", '^', 2),     // Exponentiation
        ("(", '(', 0),     // Left parenthesis
        (")", ')', 0),     // Right parenthesis
    ];
    let mut token = Token::new();

    if index < input.len() {
        for &(op_str, op_char, operands) in &operators {
            if input[index..].starts_with(op_str.as_bytes()) {
                token.operator = op_char;
                token.operands = operands;
                index += op_str.len();
                return Ok((token, index));
            }
        }
    }
    if index < input.len() && input[index] == b'#' {
        // We've encountered an unknown function
        let mut end = index + 1;
        while end < input.len() && (input[end].is_ascii_alphabetic() || input[end] == b'_') {
            end += 1;
        }
        return Err(("Unknown function!".to_owned(), index));
    }

    Ok((token, index))
}
fn parse_command(
    input: &[u8],
    mut index: usize,
    base: &mut u8,
    precision: &mut u32,
    digits: &mut usize,
    radians: &mut bool,
    colours: &RGBValues,
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

            *precision = (*digits as f64 * (new_base as f64).log2()).ceil() as u32 + 32;

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
            let mut token = Token::new();
            index = parse_number(input, &mut token, base.clone(), index + 6)?;
            // Check if there's anything after the number
            if index < input.len() {
                for i in index..input.len() {
                    if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                        return Err((format!("Invalid characters after digits value!"), i));
                    }
                }
            }

            if token.imaginary_integer.len() > 0 || token.imaginary_fraction.len() > 0 {
                return Err((format!("Precision must be a real integer!"), index));
            }

            let value = token2num(&token, *base, *precision).real().to_f64().round() as usize;

            *digits = value;
            *precision = (*digits as f64 * (*base as f64).log2()).ceil() as u32 + 32;
            message = format!(
                "Precision set to {} digits.",
                format_int(value, *base as usize)
            );
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
fn get_precedence(op: char) -> Precedence {
    match op {
        '+' | '-' => Precedence::Addition,
        '*' | '/' | '%' => Precedence::Multiplication,
        '^' => Precedence::Exponentiation,
        'n' => Precedence::UnaryNegation,
        'a' | 'C' | 'S' | 'T' | 'c' | 'i' | 'l' | 'L' | 'e' | 's' | 'q' | 't' => {
            Precedence::Function
        }
        '(' | ')' => Precedence::Highest,
        'E' | 'G' | 'g' | 'p' | 'r' => Precedence::Constant,
        _ => Precedence::Lowest,
    }
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
fn token2num(token: &Token, base: u8, precision: u32) -> Complex {
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
    let decimal_place = (num_abs.clone().log2() / (Float::with_val(num.prec(), base)).log2())
        .floor()
        .to_f64() as isize;
    num_abs = num_abs / (Float::with_val(num.prec(), base)).pow(decimal_place);
    num_abs += (Float::with_val(num.prec(), base)).pow(-(num_digits as isize)) / 2;

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

    let tilde = (num_abs - 0.5f32).abs() > 2f64.pow(-16);
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
        (":DIGits    \t__\t\t2  0.000", "Precision set to 20 digits."),
        ("5^-25", "  1.86 BA3 547 200 980 95A 405 483~ :-17"),
        (
            "5^-25*[-3.24,-4.1b]",
            "[-5.58 BA6 424 28A 6A9 238 829 279~ :-17 ,-7.17 49A 618 591 429 757 6B6 511~ :-17 ]",
        ),
        // ("-#sIn(@pi/2)", " -1."),
        // ("#sin(@pi/4)", "  8.59 A69 650 3BA 297 996 256 428~ :-1"),
        // (":deGreEs", "Angle units set to degrees."),
        // ("#sin76", "  1."),
        // (":radiAns", "Angle units set to radians."),
        ("#sin76", "  0.A88 9AB 897 724 376 B81 A25 541~"),
        ("(1+2)*3", "  9."),
        // ("--1+2*3", "  7."),
        // ("(1+2)*(3+4)", "  19."),
        // ("1+2*(3+4)", "  13."),
        // ("((1+2)*3)+4", "  11."),
        // ("1+(2*3)+4", "  B."),
        // ("2^(3^2)", "  368."),
        // ("(2^3)^2", "  54."),
        // ("#log(100)/2", "  1."),
        ("(@pi+@e)^2", "  2A.408 353 754 8B8 38B 235 632 3~"),
        ("1/(1+1/(1+1/(1+1/2)))", "  0.76"),
        // ("(((1+2)+3)+4)", "  A."),
        // ("1+(2+(3+4))", "  A."),
        // ("(1+2+3+4)", "  A."),
        // ("((())1+2(()))", "Expected number or unary operator!"),
        // ("(1+2))", "Mismatched parentheses!"),
        // ("(1+2", "Mismatched parentheses!"),
        // ("1+*2", "Expected number or unary operator!"),
        // ("1 2 + 3", "  15."),
        // ("#sin()", "Expected number or unary operator!"),
        // ("#sin", "Incomplete expression!"),
        // ("#sin(#cos())", "Expected number or unary operator!"),
        // ("1/0", "NaN"),
        // ("[0,-1]/0", "NaN"),
        (":debug", "Debug enabled"),
        ("#sqrt-1", "[ 0. , 1. ]"),
        ("#sqrt#sqrt#sqrt194", "  2."),
        ("-#cos#sin0", " -1."),
        ("#cos-#sin0", "  1."),
        ("#cos#sin-0", "  1."),
        ("---#sin---@pi", " -1."),
        (
            "#sqrt(#sqrt-1)",
            "[ 8.59 A69 650 3BA 297 996 256 428~ :-1 , 8.59 A69 650 3BA 297 996 256 428~ :-1 ]",
        ),
        (":debug", "Debug disabled"),
        ("-3", " -3."),
        ("--3", "  3."),
        ("---3", " -3."),
        ("----3", "  3."),
        ("1-3", " -2."),
        ("1--3", "  4."),
        ("1---3", " -2."),
        ("1----3", "  4."),
        ("-#sqrt4", " -2."),
        // ("1.2.3", "Multiple decimals in number!"),
        // ("#sin#cos@pi", " -A.12 08A A92 234 12B 470 074 934~ :-1"),
        // ("(1+2)*(3+4", "Mismatched parentheses!"),
        // ("#log(0)", "NaN"),
        (":debug", "Debug enabled"),
        ("#sqrt(-1-1)", "[ 0. , 1.4B7 917 0A0 7B8 573 770 4B0 85~ ]"),
        ("#sqrt-1-1", "[-1.,1]"),
        // ("1/3+1/3+1/3-1", "  0."),
        // ("@pi@e", "Expected operator!"),
        // ("#sin()#cos()", "Expected number or unary operator!"),
        // ("1++2", "Expected number or unary operator!"),
        // ("((1+2)*3", "Mismatched parentheses!"),
        // ("1+(2*3", "Mismatched parentheses!"),
        // ("1 2 3 +", "Incomplete expression!"),
        // ("1 + + 2", "Expected number or unary operator!"),
        ("#funky(1)", "Unknown function!"),
        // ("1 / (2-2)", "NaN"),
        // ("#sqrt(1+2+3)+)", "Expected number or unary operator!"),
        // ("(((1+2)*(3+4))+5", "Mismatched parentheses!"),
        // ("1 2 3 4 5", "  12 345."),
        // ("*1", "Expected number or unary operator!"),
        // ("1*", "Incomplete expression!"),
        // ("()", "Expected number or unary operator!"),
        // ("#sin", "Incomplete expression!"),
        ("123456789abcdef", "Invalid number!"),
        ("\"text in quotes\"", "invalid input!"),
        (";*&#@/\\", "invalid input!"),
    ];

    let mut passed = 0;
    let total = tests.len();

    for (input, expected) in tests {
        println!("> {}", input);

        let (coloured_result, result) = match tokenize(
            input,
            &mut base,
            &mut precision,
            &mut digits,
            &mut radians,
            colours,
        ) {
            Ok(tokens) => {
                match evaluate_tokens(&tokens, base, precision, &mut rand_state, radians) {
                    Ok(eval_value) => {
                        let coloured_vec = num2string(&eval_value, base, digits, &colours);
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
