use az::Cast;
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

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    println!("Goodbye!");
                    break;
                }
                rl.add_history_entry(line.clone())?;
                if line.starts_with(":debug") {
                    if DEBUG.load(Ordering::Relaxed) {
                        println!("Debug disabled");
                    } else {
                        println!("Debug enabled");
                    }
                    DEBUG.fetch_xor(true, Ordering::Relaxed);
                    continue;
                }

                debug_println(&format!("Processing input: '{}'", line));
                match tokenize(&line, &mut base, &mut precision, &mut digits, &mut radians) {
                    Ok(tokens) => {
                        debug_println(&format!("Tokens: {:?}", tokens));
                        let result =
                            evaluate_tokens(&tokens, base, precision, &mut rand_state, radians);
                        let result_str = num2string(&result, base, digits);
                        println!("{}", result_str);

                        debug_println(&format!("Added to history: {}", line));
                    }
                    Err((msg, pos)) => {
                        if pos == std::usize::MAX {
                            println!("{}", msg);
                        } else {
                            println!("{}\n{}^", line, " ".repeat(pos));
                            println!("Error: {}", msg);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Pressing enter with no input will exit as well.");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
static DEBUG: AtomicBool = AtomicBool::new(false);
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    Lowest,
    Addition,       // + and -
    Multiplication, // * and /
    Exponentiation, // ^
    UnaryOperator,  // Unary -, functions, etc.
    Highest,        // Parentheses
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
) -> Result<Vec<Token>, (String, usize)> {
    debug_println(&format!("Tokenizing: {}", input_str));
    let input = input_str.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut expect_value = true;
    let mut paren_count = 0;

    while index < input.len() {
        if input[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if input[index] == b':' {
            return parse_command(input, index + 1, base, precision, digits, radians);
        }

        if expect_value {
            if input[index] == b'(' {
                tokens.push(Token {
                    operator: '(',
                    operands: 0,
                    ..Token::new()
                });
                paren_count += 1;
                index += 1;
            } else if input[index] == b'#' {
                let (token, new_index) = parse_operator(input, index)?;
                tokens.push(token);
                index = new_index;
            } else {
                let mut number_token = Token::new();
                let new_index = parse_number(input, &mut number_token, base.clone(), index)?;
                debug_println(&format!("Parsed number token: {:?}", number_token));

                if number_token.operator != '\0' {
                    // This is a special number (@pi, @e, etc.)
                    tokens.push(number_token);
                    expect_value = false;
                    index = new_index;
                } else if number_token.real_integer.is_empty()
                    && number_token.real_fraction.is_empty()
                    && number_token.imaginary_integer.is_empty()
                    && number_token.imaginary_fraction.is_empty()
                {
                    return Err((format!("Expected number or unary operator!"), index));
                } else {
                    tokens.push(number_token);
                    expect_value = false;
                    index = new_index;
                }
            }
        } else {
            if input[index] == b')' {
                tokens.push(Token {
                    operator: ')',
                    operands: 0,
                    ..Token::new()
                });
                paren_count -= 1;
                index += 1;
            } else {
                let (token, new_index) = parse_operator(input, index)?;
                if token.operator != 0 as char {
                    tokens.push(token);
                    index = new_index;
                    expect_value = true;
                } else {
                    return Err((format!("Expected operator!"), index));
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

    let last_token = tokens.last().unwrap();
    if last_token.operator != 0 as char && last_token.operands > 0 {
        return Err((format!("Incomplete expression!"), input.len()));
    }

    for token in &tokens {
        debug_println(&format!("Token: {:?}", token));
    }

    Ok(tokens)
}
fn parse_number(
    input: &[u8],
    token: &mut Token,
    base: u8,
    mut index: usize,
) -> Result<usize, (String, usize)> {
    let numbers = [
        // ("operator", 'operator symbol', operands)
        ("@e", 'E', 0),     // e (Euler's number)
        ("@gamma", 'G', 0), // γ Euler-Mascheroni
        ("@grand", 'g', 0), // Gaussian random
        ("@pi", 'p', 0),    // Pi
        ("@rand", 'r', 0),  // Random
    ];
    let mut complex = false;
    let mut imaginary = false;
    let mut integer = true;
    let mut sign_check = true;
    let mut is_negative = false;
    let mut first_char = true;
    while index < input.len() {
        let mut c = input[index];
        if c == b' ' || c == b'_' || c == b'\t' {
            index += 1;
            continue;
        }
        if first_char && c == b'@' {
            for &(num_str, op_char, _) in &numbers {
                if input[index..].starts_with(num_str.as_bytes()) {
                    debug_println(&format!("Found number: {}", num_str));
                    token.operator = op_char;
                    index += num_str.len();
                    return Ok(index);
                }
            }
            return Err((format!("Invalid @number!"), index));
        }
        first_char = false;
        if sign_check {
            if c == b'[' {
                if !(token.real_integer.is_empty()
                    && token.real_fraction.is_empty()
                    && token.imaginary_integer.is_empty()
                    && token.imaginary_fraction.is_empty())
                {
                    return Err((format!("Expected operator!"), index));
                }
                complex = true;
                index += 1
            }
            is_negative = input[index] == b'-';
            index += is_negative as usize;
            c = input[index];
        }
        sign_check = false;
        if c.is_ascii_digit() || c.is_ascii_alphabetic() {
            let num;
            if c.is_ascii_digit() {
                num = c - b'0';
            } else if c.is_ascii_uppercase() {
                num = c - b'A' + 10;
            } else {
                num = c - b'a' + 10;
            }
            if num >= base {
                return Err((format!("Invalid number!"), index));
            }
            if imaginary {
                if integer {
                    token.sign.1 = is_negative;
                    token.imaginary_integer.push(num);
                } else {
                    token.imaginary_fraction.push(num)
                }
            } else {
                if integer {
                    token.sign.0 = is_negative;
                    token.real_integer.push(num);
                } else {
                    token.real_fraction.push(num)
                }
            }
            index += 1;
        } else if c == b',' {
            if complex {
                if token.real_integer.is_empty() && token.real_fraction.is_empty() {
                    return Err((format!("Missing real value!"), index));
                }
                imaginary = true;
                sign_check = true;
                integer = true;
                index += 1;
            } else {
                return Err((
                    format!("Commas allowed for complex number entry only!"),
                    index,
                ));
            }
        } else if c == b'[' {
            if !(token.real_integer.is_empty()
                && token.real_fraction.is_empty()
                && token.imaginary_integer.is_empty()
                && token.imaginary_fraction.is_empty())
            {
                return Err((format!("Expected operator!"), index));
            }
            complex = true;
            index += 1
        } else if c == b']' {
            if !complex {
                return Err((format!("Missing opening brackets!"), index));
            }
            if token.imaginary_integer.is_empty() && token.imaginary_fraction.is_empty() {
                return Err((format!("Missing imaginary value!"), index));
            }
            return Ok(index + 1);
        } else if c == b'.' {
            if integer {
                integer = false;
                index += 1
            } else {
                return Err((format!("Multiple decimals in number!"), index));
            }
        } else {
            return Ok(index);
        }
    }
    if complex {
        return Err((format!("Missing closing brackets!"), index));
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
        ("#im", 'i', 1),   // Imaginary
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
) -> Result<Vec<Token>, (String, usize)> {
    let message;
    match &input[index..] {
        s if s.eq_ignore_ascii_case(b"test") => {
            let (passed, total) = run_tests();
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
        s if s.len() >= 9 && s[..9].eq_ignore_ascii_case(b"precision") => {
            let mut token = Token::new();
            index = parse_number(input, &mut token, base.clone(), index + 9)?;
            // Check if there's anything after the number
            if index < input.len() {
                for i in index..input.len() {
                    if input[i] != b' ' && input[i] != b'_' && input[i] != b'\t' {
                        return Err((format!("Invalid characters after precision value!"), i));
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
fn apply_operator(
    output_queue: &mut Vec<Complex>,
    op: char,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    base: u8,
    radians: bool,
) {
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
            // Unary negation
            if let Some(operand) = output_queue.pop() {
                debug_println(&format!("Result after operation: {:?}", -operand.clone()));
                output_queue.push(-operand);
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
                    _ => panic!("Unknown operator: {}", op),
                };
                debug_println(&format!("Result after operation: {:?}", result));
                output_queue.push(result);
            }
        }
    }
}

fn evaluate_tokens(
    tokens: &[Token],
    base: u8,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    radians: bool,
) -> Complex {
    let mut output_queue: Vec<Complex> = Vec::new();
    let mut operator_stack: Vec<char> = Vec::new();

    for token in tokens {
        debug_println(&format!("Processing token: {:?}", token));
        if token.operator == '\0' {
            let value = token2num(token, base, precision);
            debug_println(&format!("  Pushed number: {}", value));
            output_queue.push(value);
        } else if token.operator == 'E'
            || token.operator == 'G'
            || token.operator == 'p'
            || token.operator == 'r'
            || token.operator == 'g'
        {
            // Handle special number operators
            apply_operator(
                &mut output_queue,
                token.operator,
                precision,
                rand_state,
                base,
                radians,
            );
        } else {
            match token.operator {
                '(' => operator_stack.push('('),
                ')' => {
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
                        );
                    }
                    if operator_stack.pop() != Some('(') {
                        panic!("Mismatched parentheses");
                    }
                    // Apply function operator if it's on top of the stack
                    if let Some(&op) = operator_stack.last() {
                        if get_precedence(op) == Precedence::UnaryOperator {
                            apply_operator(
                                &mut output_queue,
                                operator_stack.pop().unwrap(),
                                precision,
                                rand_state,
                                base,
                                radians,
                            );
                        }
                    }
                }
                _ => {
                    // Handle both unary and binary operators
                    while !operator_stack.is_empty() {
                        let top_op = *operator_stack.last().unwrap();
                        if top_op == '(' {
                            break;
                        }
                        if (get_precedence(top_op) > get_precedence(token.operator))
                            || (get_precedence(top_op) == get_precedence(token.operator)
                                && token.operator != 'n')
                        // Allow 'n' (unary minus) to be right-associative
                        {
                            apply_operator(
                                &mut output_queue,
                                operator_stack.pop().unwrap(),
                                precision,
                                rand_state,
                                base,
                                radians,
                            );
                        } else {
                            break;
                        }
                    }
                    operator_stack.push(token.operator);
                }
            }
        }
    }

    while let Some(op) = operator_stack.pop() {
        if op == '(' {
            panic!("Mismatched parentheses");
        }
        apply_operator(&mut output_queue, op, precision, rand_state, base, radians);
    }

    if output_queue.len() != 1 {
        panic!("Invalid expression!");
    }

    output_queue.pop().unwrap()
}
fn get_precedence(op: char) -> Precedence {
    match op {
        '+' | '-' => Precedence::Addition,
        '*' | '/' | '%' => Precedence::Multiplication,
        '^' => Precedence::Exponentiation,
        'a' | 'C' | 'S' | 'T' | 'c' | 'i' | 'l' | 'L' | 'e' | 's' | 'q' | 't' | 'n' => {
            Precedence::UnaryOperator
        }
        '(' | ')' => Precedence::Highest,
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
fn num2string(num: &Complex, base: u8, digits: usize) -> String {
    if num.real().is_nan()
        || num.imag().is_nan()
        || num.real().is_infinite()
        || num.imag().is_infinite()
    {
        return "NaN".to_string();
    }

    let number;
    if num.imag().is_zero() {
        number = format!(" {}", format_part(num.real(), base, digits));
    } else {
        number = format!(
            "[{} ,{} ]",
            format_part(num.real(), base, digits),
            format_part(num.imag(), base, digits)
        );
    };
    number
}
fn format_part(num: &rug::Float, base: u8, num_digits: usize) -> String {
    if num.is_zero() {
        return " 0.".to_owned();
    }
    if num.is_nan() || num.is_infinite() {
        return "NaN".to_owned();
    }
    let mut number = "".to_owned();

    let is_positive = num.is_sign_positive();
    let mut num_abs = num.clone().abs();
    let decimal_place = (num_abs.clone().log2() / (Float::with_val(num.prec(), base)).log2())
        .floor()
        .to_f64() as isize;
    num_abs = num_abs / (Float::with_val(num.prec(), base)).pow(decimal_place);
    num_abs += (Float::with_val(num.prec(), base)).pow(-(num_digits as isize)) / 2;
    let mut decimal = false;
    for digit_number in 0..num_digits {
        let mut digit: u8 = num_abs.clone().floor().cast();

        num_abs = num_abs - digit;
        num_abs *= base;
        if digit < 10 {
            digit += b'0'
        } else {
            digit += b'A' - 10
        }
        number.push(digit as char);
        let offset = digit_number as isize - decimal_place;
        if offset == 0 {
            number.push('.');
            decimal = true;
        } else if offset % 3 == 0 && digit_number != 0 && digit_number != num_digits - 1 {
            number.push(' ')
        }
    }
    if (num_abs - 0.5f32).abs() > 2f64.pow(-16) {
        number.push('~');
    } else {
        let mut index = number.len() - 1;
        while index > 0 {
            if number.as_bytes()[index] != b'0' && number.as_bytes()[index] != b' ' {
                break;
            }
            index -= 1;
        }
        number.truncate(index + 1);
    }

    if !decimal {
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
        number.push_str(" :");
        if decimal_place < 0 {
            number.push('-');
            number.push_str(&format_int((-decimal_place) as usize, base as usize));
        } else {
            number.push(' ');
            number.push_str(&format_int(decimal_place as usize, base as usize));
        }
    }
    if is_positive {
        format!(" {}", number)
    } else {
        format!("-{}", number)
    }
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
use colored::Colorize;
fn run_tests() -> (usize, usize) {
    let mut base = 10;
    let mut digits = 12;
    let mut precision = (digits as f64 * (base as f64).log2()).ceil() as u32 + 32;
    let mut radians = true;
    let mut rand_state = rand::RandState::new();

    let tests = vec![
        (":base C", "Base set to Dozenal (C)."),
        (":precision 20", "Precision set to 20 digits."),
        ("#sin(@pi/2)", "  1."),
        ("#sin(@pi/4)", "  8.59 A69 650 3BA 297 996 256 428~ :-1"),
        (":degrees", "Angle units set to degrees."),
        ("#sin76", "  1."),
        (":radians", "Angle units set to radians."),
        ("#sin76", "  A.88 9AB 897 724 376 B81 A25 541~ :-1"),
        ("(1+2)*3", "  9."),
        ("1+2*3", "  7."),
        ("(1+2)*(3+4)", "  19."),
        ("1+2*(3+4)", "  13."),
        ("((1+2)*3)+4", "  11."),
        ("1+(2*3)+4", "  B."),
        ("2^(3^2)", "  368."),
        ("(2^3)^2", "  54."),
        ("#log(100)/2", "  1."),
        ("(@pi+@e)^2", "  2A.408 353 754 8B8 38B 235 632 3~"),
        ("1/(1+1/(1+1/(1+1/2)))", "  7.6 :-1"),
        ("(((1+2)+3)+4)", "  A."),
        ("1+(2+(3+4))", "  A."),
        ("(1+2+3+4)", "  A."),
        ("((())1+2(()))", "Expected number or unary operator!"),
        ("(1+2))", "Mismatched parentheses!"),
        ("(1+2", "Mismatched parentheses!"),
        ("1+*2", "Expected number or unary operator!"),
        ("1 2 + 3", "  15."),
        ("#sin()", "Expected number or unary operator!"),
        ("#sin", "Incomplete expression!"),
        ("#sin(#cos())", "Expected number or unary operator!"),
        ("1/0", "NaN"),
        ("[0,-1]/0", "NaN"),
        ("#sqrt-1", "[ 0. , 1. ]"),
        ("---#sin---@pi", " -1."),
        ("#sqrt(#sqrt-1)", "  8.59 A69 650 3BA 297 996 256 428~ :-1"),
        ("-3", " -3."),
        ("--3", " 3."),
        ("---3", " -3."),
        ("----3", " 3."),
        ("1-3", " -2."),
        ("1--3", "  4."),
        ("1---3", " -2."),
        ("1----3", "  4."),
        ("1.2.3", "Multiple decimals in number!"),
        ("#sin#cos@pi", " -A.12 08A A92 234 12B 470 074 934~ :-1"),
        ("(1+2)*(3+4", "Mismatched parentheses!"),
        ("#log(0)", "NaN"),
        ("#sqrt(-1-1)", "[ 0. , 1.4B7 917 0A0 7B8 573 770 4B0 85~ ]"),
        ("1/3+1/3+1/3-1", "  0."),
        ("@pi@e", "Expected operator!"),
        ("#sin()#cos()", "Expected number or unary operator!"),
        ("1++2", "Expected number or unary operator!"),
        ("((1+2)*3", "Mismatched parentheses!"),
        ("1+(2*3", "Mismatched parentheses!"),
        ("1 2 3 +", "Incomplete expression!"),
        ("1 + + 2", "Expected number or unary operator!"),
        ("#funky(1)", "Unknown function!"),
        ("1 / (2-2)", "NaN"),
        ("#sqrt(1+2+3)+)", "Expected number or unary operator!"),
        ("(((1+2)*(3+4))+5", "Mismatched parentheses!"),
        ("1 2 3 4 5", "  12 345."),
        ("*1", "Expected number or unary operator!"),
        ("1*", "Incomplete expression!"),
        ("()", "Expected number or unary operator!"),
        ("#sin", "Incomplete expression!"),
    ];

    let mut passed = 0;
    let total = tests.len();

    for (input, expected) in tests {
        println!("Test input: '{}'", input);
        let result = match tokenize(input, &mut base, &mut precision, &mut digits, &mut radians) {
            Ok(tokens) => {
                let eval_result =
                    evaluate_tokens(&tokens, base, precision, &mut rand_state, radians);
                num2string(&eval_result, base, digits)
            }
            Err((msg, _pos)) => msg.to_string(),
        };

        println!("Result    : {}", result);
        println!("Expected  : {}", expected);

        if result == expected {
            println!("{}", "Test passed!".green());
            passed += 1;
        } else {
            println!("{}", "Test failed!".red());
        }
        println!();
    }

    (passed, total)
}
