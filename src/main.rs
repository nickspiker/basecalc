use az::Cast;
use rug::ops::*;
use rug::*;
use rustyline::{
    error::ReadlineError, history::FileHistory, Cmd, Config, Editor, KeyCode, KeyEvent, Modifiers,
};
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

fn main() {
    let config = Config::builder().history_ignore_space(true).build();

    let mut rl = Editor::<(), FileHistory>::with_config(config).expect("Failed to create editor");
    // Bind up arrow to reverse search
    rl.bind_sequence(
        KeyEvent(KeyCode::Up, Modifiers::NONE),
        Cmd::ReverseSearchHistory,
    );
    let mut number_history = Vec::new();
    let mut base = 17;
    let mut digits = 256;
    let mut precision = (digits as f64 * (base as f64).log2()).ceil() as u32 + 32; // 32 ensures answer int/float detection within a reasonable amount
    let mut number = Complex::new(precision);
    let time = chrono::Utc::now();
    let time1 = time.timestamp().to_le_bytes();
    let time2 = time.timestamp_subsec_nanos().to_le_bytes();
    let mut forhash = time1.to_vec();
    forhash.append(&mut time2.to_vec());
    let mut salt = vec![
        0x1B, 0xE5, 0xAF, 0x17, 0x64, 0xAD, 0xE7, 0x7C, 0xDA, 0xC1, 0x59, 0xA9, 0xE0, 0xEF, 0x6C,
        0x93, 0xFD, 0xED, 0xB6, 0x54, 0x47, 0x25, 0xF6, 0x89, 0x77, 0x06, 0x43, 0xE2, 0x15, 0x5E,
        0xEE, 0x8C,
    ];
    forhash.append(&mut salt);
    let mut rand_state = rand::RandState::new();
    let mut seed = Integer::new();
    for byte in blake3::hash(&forhash).as_bytes() {
        seed *= 256;
        seed += byte;
    }
    rand_state.seed(&seed);

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
                        evaluate_tokens(&mut number, &tokens, base, precision, &mut rand_state);
                        let result_str;
                        if number.real().is_finite() && number.imag().is_finite() {
                            result_str = num2string(&number, base, digits);
                        } else {
                            number = Complex::with_val(precision, std::f32::NAN);
                            result_str = "Undefined!".to_owned();
                        }
                        number_history.push(number.clone());
                        println!("{}", &result_str);
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
    // if input[0] == b':' || input[0] == b'@' {
    //     let (mut token, new_index) = parse_operator(input, index)?;// then parse nummber as int and check if index is at end before whitespace
    // }
    while index < input.len()
        && (input[index] == b' ' || input[index] == b'_' || input[index] == b'\t')
    {
        index += 1;
    }
    let (mut token, new_index) = parse_operator(input, index)?;
    if token.operator != 0 as char {
        index = new_index;
    }
    while index < input.len() {
        if input[index] == b' ' || input[index] == b'_' || input[index] == b'\t' {
            index += 1;
            continue;
        }
        if token.operands == 2 {
            let new_index = parse_number(input, &mut token, base, index)?;
            if token.real_integer.is_empty()
                && token.real_fraction.is_empty()
                && token.imaginary_integer.is_empty()
                && token.imaginary_fraction.is_empty()
            {
                return Err((format!("Missing number!"), index));
            }
            index = new_index;
        }
        tokens.push(token);
        let (new_token, new_index) = parse_operator(input, index)?;
        token = new_token;
        if token.operator == 0 as char {
            if index == input.len() {
                break;
            }
            return Err((format!("Missing operator!"), index));
        }
        index = new_index;
    }
    if token.operator != 0 as char {
        if token.operands == 2 {
            return Err((format!("Missing number!"), index));
        }
        tokens.push(token);
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
    let mut sign_check = true;
    let mut is_negative = false;
    while index < input.len() {
        // add code to check operator, if base, history entry or constant, encode as integer
        let mut c = input[index];
        if c == b' ' || c == b'_' || c == b'\t' {
            index += 1;
            continue;
        }
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
        ("#abs", 'a', 1),       // Absolute value
        ("#acos", 'C', 1),      // Inverse cosine
        ("#asin", 'S', 1),      // Inverse sine
        ("#atan", 'T', 1),      // Inverse tangent
        ("#cos", 'c', 1),       // Cosine
        ("#im", 'i', 1),        // Imaginary
        ("#ln", 'l', 1),        // Natural logarithm
        ("#log", 'L', 1),       // Base logarithm
        ("#rand", 'r', 1),      // Random
        ("#re", 'e', 1),        // Real
        ("#sin", 's', 1),       // Sine
        ("#sqrt", 'q', 1),      // Square root
        ("#tan", 't', 1),       // Tangent
        ("%", '%', 2),          // Moduland, modular order
        ("$", '$', 2),          // Modulor, moduland order
        ("&", '&', 2),          // Exponentiation
        ("*", '*', 2),          // Multiplication
        ("+", '+', 2),          // Addition
        ("-", '-', 2),          // Subtraction
        ("/", '/', 2),          // Dividend divisor order
        (":base", 'b', 0),      // Sets base to any base from 2 to 36
        (":degrees", 'b', 0),   // Sets trig units to degrees
        (":precision", 'p', 0), // Sets precision in digits in given base plus 32 bits of padding
        (":radians", 'b', 0),   // Sets trig units to radians
        ("?", '@', 0),          // History entry
        ("@pi", 'p', 0),        // Pi
        ("@e", 'e', 0),         // e
        ("^", '^', 2),          // Exponentiation
        ("\\", '\\', 2),        // Divisor dividend order
    ];

    let mut token = Token::new();
    let mut low = 0;
    let mut high = operators.len() - 1;
    let mut op_index = 0;

    while low <= high && index < input.len() {
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
            }
            if low >= operators.len() {
                break;
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
            token.operands = operators[low].2;
            break;
        }
    }
    Ok((token, index))
}
fn evaluate_tokens(
    number: &mut Complex,
    tokens: &[Token],
    base: u8,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
) {
    for token in tokens {
        let token_number = token2num(token, base, precision);
        match token.operator {
            '\0' => *number = token_number.clone(), // Clear register and load number
            'a' => *number = number.clone().abs(),  // Absolute value
            'C' => *number = number.clone().acos(), // Inverse cosine
            'S' => *number = number.clone().asin(), // Inverse sine
            'T' => *number = number.clone().atan(), // Inverse tangent
            'c' => *number = number.clone().cos(),  // Cosine
            'i' => *number = Complex::with_val(precision, (number.clone().imag(), 0)), // Imaginary
            'l' => *number = number.clone().ln(),   // Natural logarithm
            'L' => *number = number.clone().ln() / Float::with_val(precision, base).ln(), // Base logarithm
            'r' => *number = generate_random(precision, rand_state),
            'e' => *number = Complex::with_val(precision, (number.clone().real(), 0)), // Real
            's' => *number = number.clone().sin(),                                     // Sine
            'q' => *number = number.clone().sqrt(), // Square Root
            't' => *number = number.clone().tan(),  // Tangent
            '%' => *number = number.modulus(token_number), // Moduland, modular order
            '$' => *number = token_number.modulus(number.clone()), // Modulor, moduland order
            '&' => *number = number.clone().pow(&token_number), // Exponentiation
            '*' => *number *= &token_number,        // Multiplication
            '+' => *number += &token_number,        // Addition
            '-' => *number -= &token_number,        // Subtraction
            '/' => *number /= &token_number,        // Dividend divisor order
            '\\' => *number = &token_number / number.clone(), // Divisor dividend order
            //     'p' => set_precision(token, base), // Sets precision in digits in given base plus 32 bits of padding
            //    'b' => set_base(token, base),      // Sets base to any base from 2 to Z+1
            '^' => *number = number.clone().pow(&token_number), // Exponentiation
            _ => panic!("Unknown operator!"),
        }
    }
}

fn set_precision(token: &Token, base: u8) {
    let mut precision = 0;
    for &digit in &token.real_integer {
        precision *= base;
        precision += digit;
    }
    for &digit in &token.real_fraction {
        precision *= base;
        precision += digit;
    }
    precision += 32;
    //   number.set_prec(precision);
}

fn generate_random(precision: u32, rand_state: &mut rug::rand::RandState) -> Complex {
    let real_sign = Float::with_val(1, Float::random_cont(rand_state));
    let real = if real_sign > 0.375 {
        Float::with_val(precision, Float::random_cont(rand_state))
    } else {
        -Float::with_val(precision, Float::random_cont(rand_state))
    };
    let imag_sign = Float::with_val(1, Float::random_cont(rand_state));
    let imaginary = if imag_sign > 0.375 {
        Float::with_val(precision, Float::random_cont(rand_state))
    } else {
        -Float::with_val(precision, Float::random_cont(rand_state))
    };
    Complex::with_val(precision, (real, imaginary))
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
