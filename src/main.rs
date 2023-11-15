use az::Cast;
use rug::ops::*;
use rug::*;
use rustyline::{
    error::ReadlineError, history::FileHistory, Cmd, Config, Editor, KeyCode, KeyEvent, Modifiers,
};
struct Token {
    operator: char,
    second_operand: bool,
    real_integer: Vec<u8>,
    real_fraction: Vec<u8>,
    imaginary_integer: Vec<u8>,
    imaginary_fraction: Vec<u8>,
}
impl Token {
    // Define a new function to create a Token instance
    fn new() -> Token {
        Token {
            operator: 0 as char,
            second_operand: true,
            real_integer: Vec::new(),
            real_fraction: Vec::new(),
            imaginary_integer: Vec::new(),
            imaginary_fraction: Vec::new(),
        }
    }
}
fn main() {
    let config = Config::builder().history_ignore_space(true).build();

    let mut rl = Editor::<(), FileHistory>::with_config(config).expect("Failed to create editor");
    // Bind up arrow to reverse search
    rl.bind_sequence(
        KeyEvent(KeyCode::Up, Modifiers::NONE),
        Cmd::ReverseSearchHistory,
    );

    let mut number_history = Vec::new();
    let mut base = 12;
    let mut digits = 256;
    let mut precision = (digits as f64 * (base as f64).log2()).ceil() as u32 + 32; // 32 ensures answer int/float detection within a reasonable amount
    let mut number = Complex::new(precision);

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    break;
                }
                let tokens = tokenize(&line, base);
                match tokens {
                    Ok(tokens) => {
                        evaluate_tokens(&mut number, &tokens, base, precision);
                        let result_str = num2string(&number, base, digits);
                        number_history.push(number.clone());
                        println!("  {}", result_str);
                        rl.add_history_entry(line.as_str())
                            .expect("Unable to store entry to history!");
                        rl.add_history_entry(result_str)
                            .expect("Unable to store result to history!");
                    }
                    Err(e) => {
                        print!("  ");
                        for _ in 0..e.1 {
                            print!(" ")
                        }
                        println!("^");
                        println!("Error: {}", e.0);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("You can always press enter with no input to exit the program");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D. Exiting program.");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

/// Tokenizes the given input string into a vector of tuples. Each tuple contains two vectors of bytes:
/// one for the operator and the integer portion of the number, and another for the fractional part of the number.
/// This function handles arithmetic operators and supports numbers in arbitrary bases.
/// It returns the first error it finds and position of the error in case of invalid input.
///
/// # Arguments
/// * `input_str` - A string slice that holds the string to tokenize.
///
/// # Returns
/// A `Result` containing either:
/// * `Ok(Vec<(Vec<u8>, Vec<u8>)>)` - A vector of tuples with tokenized operators, integer portions, and fractional portions of numbers, or
/// * `Err((String, usize))` - An error message and the position in the input string where the error occurred.
fn tokenize(input_str: &str, base: u8) -> Result<Vec<Token>, (String, usize)> {
    let input = input_str.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut first_symbol = true;
    while index < input.len() {
        let a = input[index];
        if a == b' ' || a == b'_' || a == b'\t' {
            index += 1;
            continue;
        }
        let (mut token, new_index) = parse_operator(input, base, index)?;
        if token.operator == 0 as char && first_symbol {
            index = parse_number(input, &mut token, base, index)?;
            if token.real_integer.is_empty()
                && token.real_fraction.is_empty()
                && token.imaginary_integer.is_empty()
                && token.imaginary_fraction.is_empty()
            {
                return Err((format!("Unrecognized input!"), index));
            } else {
                tokens.push(token);
            }
        } else {
            index = new_index;
            if token.second_operand {
                let old_index = parse_number(input, &mut token, base, index)?;
                if token.real_integer.is_empty()
                    && token.real_fraction.is_empty()
                    && token.imaginary_integer.is_empty()
                    && token.imaginary_fraction.is_empty()
                {
                    return Err((format!("Missing operand!"), index));
                }
                index = old_index;
            }
            tokens.push(token);
        }
        first_symbol = false;
    }
    Ok(tokens)
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
    while index < input.len() {
        let c = input[index];
        if c == b' ' || c == b'_' || c == b'\t' {
            index += 1;
            continue;
        }
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
                    token.imaginary_integer.push(num);
                } else {
                    token.imaginary_fraction.push(num)
                }
            } else {
                if integer {
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
                integer = true;
                index += 1;
            } else {
                return Err((
                    format!("Commas allowed for complex number entry only!"),
                    index,
                ));
            }
        } else if c == b'[' {
            complex = true;
            index += 1
        } else if c == b']' {
            if token.imaginary_integer.is_empty() && token.imaginary_fraction.is_empty() {
                return Err((format!("Missing imaginary value!"), index));
            }
            complex = false;
            imaginary = false;
            integer = true;
            index += 1
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
        return Err((format!("Missing closing parenthesis!"), index));
    }
    Ok(index)
}
fn parse_operator(
    input: &[u8],
    base: u8,
    mut index: usize,
) -> Result<(Token, usize), (String, usize)> {
    let operators = [
        // (Text entry, Operator, Number of operands)
        // Operators must be sorted in ASCII order!
        // ("", 0, true),           // Clear register and load number
        ("!", '!', false),         // Factorial
        ("#abs", 'a', false),      // Absolute value
        ("#acos", 'C', false),     // Arc cosine
        ("#asin", 'S', false),     // Arc sine
        ("#atan", 'T', false),     // Arc tangent
        ("#cos", 'c', false),      // Cosine
        ("#erf", 'r', false),      // Error function
        ("#exp", 'e', false),      // Exponential function
        ("#ln", 'l', false),       // Natural logarithm
        ("#sin", 's', false),      // Sine
        ("#sqrt", 'q', false),     // Square root
        ("#tan", 't', false),      // Tangent
        ("%", '%', true),          // Modulo
        ("*", '*', true),          // Multiplication
        ("+", '+', true),          // Addition
        ("-", '-', true),          // Subtraction
        ("/", '/', true),          // Division
        (":precision", 'p', true), // Sets precision in digits in given base plus 32 bits of padding
        (":base", 'b', true),      // Sets base to any base from 2 to 36
        ("@", '@', true),          // History entry
        ("^", '^', true),          // Exponentiation
    ];

    let mut token = Token::new();
    let mut low = 0;
    let mut high = operators.len() - 1;
    let mut op_index = 0;

    while low <= high {
        let c;
        if index < input.len() {
            c = input[index]
        } else {
            break;
        }
        if c == b' ' || c == b'_' || c == b'\t' {
            index += 1;
            continue;
        }
        loop {
            if op_index < operators[low].0.len() {
                let op_char = operators[low].0.as_bytes()[op_index];
                if c > op_char {
                    low += 1;
                } else {
                    break;
                }
            } else {
                low += 1;
                if low >= operators.len() {
                    return Err((format!("Invalid operator!"), index));
                }
            }
        }
        loop {
            if op_index < operators[high].0.len() {
                let op_char = operators[high].0.as_bytes()[op_index];
                if c < op_char {
                    if high == 0 {
                        return Err((format!("Invalid operator!"), index));
                    }
                    high -= 1;
                } else {
                    break;
                }
            } else {
                if high == 0 {
                    break;
                }
                high -= 1;
            }
        }
        index += 1;
        op_index += 1;
        if low == high && op_index == operators[low].0.len() {
            // Found operator
            token.operator = operators[low].1;
            token.second_operand = operators[low].2;
            break;
        }
    }
    Ok((token, index))
}
fn evaluate_tokens(number: &mut Complex, tokens: &[Token], base: u8, precision: u32) {
    for token in tokens {
        let token_number = token2num(token, base, precision);
        match token.operator {
            '\0' => *number = token_number.clone(),
            'a' => *number = number.clone().abs(), // Absolute value
            'S' => *number = number.clone().asin(), // Arc Sine
            'C' => *number = number.clone().acos(), // Arc Cosine
            'T' => *number = number.clone().atan(), // Arc Tangent
            's' => *number = number.clone().cos(), // Sine
            'c' => *number = number.clone().cos(), // Cosine
            't' => *number = number.clone().tan(), // Tangent
            'e' => *number = number.clone().exp(), // Exponential
            'l' => *number = number.clone().ln(),  // Natural Logarithm
            'L' => *number = number.clone().ln(),  // Current Base Logarithm
            'q' => *number = number.clone().sqrt(), // Square Root
            '%' => {
                // Modulus
            }
            '*' => *number *= &token_number, // Multiplication
            '+' => *number += &token_number, // Addition
            '-' => *number -= &token_number, // Subtraction
            '/' => *number /= &token_number, // Division
            '^' => *number = number.clone().pow(&token_number), // Exponentiation
            '!' => {
                // Factorial is not directly supported for Complex in `rug`.
                // You need to implement this or handle it separately.
            }
            _ => panic!("Unknown operator!"),
        }
    }
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

    Complex::with_val(precision, (real_int + real_frac, imag_int + imag_frac))
}

fn num2string(num: &Complex, base: u8, digits: usize) -> String {
    let mut number;
    if num.imag().is_zero() {
        number = " ".to_owned();
        number = format_part(num.real(), base, digits, number);
    } else {
        number = "[".to_owned();
        number = format_part(num.real(), base, digits, number);
        number.push(',');
        number = format_part(num.imag(), base, digits, number);
        number.push(']');
    };
    number
}
fn format_part(num: &rug::Float, base: u8, num_digits: usize, mut number: String) -> String {
    if num.is_zero() {
        number.push_str(" 0.");
        return number;
    }
    let mut num_abs;
    if num.is_sign_positive() {
        num_abs = num.clone();
        number.push(' ');
    } else {
        num_abs = -num.clone();
        number.push('-');
    }
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
        } else if offset % 3 == 0 {
            number.push(' ')
        }
    }
    if (num_abs - 0.5f32).abs() > 2f64.pow(-16) {
        number.push('~');
    }
    if let Some(trim_pos) = number
        .as_bytes()
        .iter()
        .rev()
        .position(|&c| c != b'0' && c != b' ')
    {
        number.truncate(number.len() - trim_pos);
    }
    if !decimal {
        let header_length = 2;
        let first = number.as_bytes()[header_length];
        let is_space = first == b' ';
        if is_space {
            let mut new_number = "".to_owned();
            new_number.push(number.as_bytes()[header_length + 1] as char);
            new_number.push('.');
            new_number.push_str(number.split_at(header_length + 2).1);
            number = new_number;
        } else {
            let mut new_number = "".to_owned();
            new_number.push(first as char);
            new_number.push('.');
            new_number.push_str(number.split_at(header_length + 1).1);
            number = new_number;
        }
        number.push(':');
        if decimal_place < 0 {
            number.push('-');
            number.push_str(&format_int((-decimal_place) as usize, base as usize));
        } else {
            number.push_str(&format_int(decimal_place as usize, base as usize));
        }
    }
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
