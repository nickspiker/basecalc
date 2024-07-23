use az::Cast;
use rug::ops::*;
use rug::*;
use rustyline::{
    error::ReadlineError, history::FileHistory, Cmd, Config, Editor, KeyCode, KeyEvent, Modifiers,
};
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Lowest,
    Addition,       // + and -
    Multiplication, // * and /
    Exponentiation, // ^
    Function,       // sin, cos, etc.
    UnaryMinus,     // Unary -
}
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
    let mut base = 10;
    let mut digits = 12;
    let mut precision = (digits as f64 * (base as f64).log2()).ceil() as u32 + 32; // 32 ensures answer int/float detection within a reasonable amount
    let mut radians = true;
    let mut number;
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
                let tokens = tokenize(&line, &mut base, &mut precision, &mut digits, &mut radians);
                match tokens {
                    Ok(tokens) => {
                        number = evaluate_tokens(&tokens, base, precision, &mut rand_state);
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
                        if e.1 == std::usize::MAX {
                            println!("{}", e.0);
                        } else {
                            print!("  ");
                            for _ in 0..e.1 {
                                print!(" ")
                            }
                            println!("^");
                            println!("Error: {}", e.0);
                        }
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
fn tokenize(
    input_str: &str,
    base: &mut u8,
    precision: &mut u32,
    digits: &mut usize,
    radians: &mut bool,
) -> Result<Vec<Token>, (String, usize)> {
    let input = input_str.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut expect_number = true;

    while index < input.len() {
        if input[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if input[index] == b':' {
            return parse_command(input, index + 1, base, precision, digits, radians);
        }

        if expect_number {
            if input[index] == b'(' {
                tokens.push(Token {
                    operator: '(',
                    operands: 0,
                    ..Token::new()
                });
                index += 1;
            } else {
                let (token, new_index) = parse_operator(input, index)?;
                if token.operator != 0 as char {
                    if token.operands == 1 || token.operands == 0 {
                        // Unary operator or constant
                        let is_constant = token.operands == 0;
                        tokens.push(token);
                        index = new_index;
                        if is_constant {
                            expect_number = false;
                        }
                    } else {
                        return Err((format!("Expected number, found operator"), index));
                    }
                } else {
                    let mut number_token = Token::new();
                    let new_index = parse_number(input, &mut number_token, base.clone(), index)?;
                    if number_token.real_integer.is_empty()
                        && number_token.real_fraction.is_empty()
                        && number_token.imaginary_integer.is_empty()
                        && number_token.imaginary_fraction.is_empty()
                    {
                        return Err((format!("Missing number!"), index));
                    }
                    tokens.push(number_token);
                    expect_number = false;
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
                index += 1;
                expect_number = false;
            } else {
                let (token, new_index) = parse_operator(input, index)?;
                if token.operator != 0 as char {
                    if token.operands == 2 {
                        tokens.push(token);
                        expect_number = true;
                        index = new_index;
                    } else {
                        return Err((
                            format!("Expected binary operator, found unary operator or constant"),
                            index,
                        ));
                    }
                } else {
                    // Implicit multiplication (e.g., 2(3+4) or 2@pi)
                    let mut times_token = Token::new();
                    times_token.operator = '*';
                    times_token.operands = 2;
                    tokens.push(times_token);
                    expect_number = true;
                }
            }
        }
    }

    if tokens.is_empty() {
        return Err((format!("Empty expression"), 0));
    }

    let last_token = tokens.last().unwrap();
    if last_token.operator != 0 as char && last_token.operands > 0 {
        return Err((format!("Incomplete expression"), input.len()));
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
        ("?", '@', 0),     // History entry
        ("@pi", 'p', 0),   // Pi
        ("@e", 'E', 0),    // e (Euler's number)
        ("@rand", 'r', 0), // Random
        ("^", '^', 2),     // Exponentiation
        ("(", '(', 0),     // Left parenthesis
        (")", ')', 0),     // Right parenthesis
    ];

    let mut token = Token::new();
    let mut low = 0;
    let mut high = operators.len() - 1;
    let mut op_index = 0;

    while low <= high && index < input.len() {
        let c = input[index] as char;
        if c.is_whitespace() {
            index += 1;
            continue;
        }

        while low < operators.len() && op_index < operators[low].0.len() {
            let op_char = operators[low].0.as_bytes()[op_index] as char;
            if c > op_char {
                low += 1;
            } else {
                break;
            }
        }

        while high > 0 && op_index < operators[high].0.len() {
            let op_char = operators[high].0.as_bytes()[op_index] as char;
            if c < op_char {
                high -= 1;
            } else {
                break;
            }
        }

        if low > high {
            break;
        }

        index += 1;
        op_index += 1;

        if low == high && op_index == operators[low].0.len() {
            // Found operator
            token.operator = operators[low].1;
            token.operands = operators[low].2;

            // Special handling for constants and parentheses
            match operators[low].0 {
                "@pi" | "@e" | "@rand" => {
                    // Treat constants as numbers (operands = 0)
                    token.operands = 0;
                }
                "(" | ")" => {
                    // Parentheses are special cases, handle accordingly
                    token.operands = 0;
                }
                _ => {}
            }

            break;
        }
    }

    if token.operator == 0 as char {
        Ok((token, index))
    } else {
        Ok((token, index))
    }
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
                    format!("Base must be a single digit!\nUse ':base 0' for base Z+1"),
                    index,
                ));
            }
            if new_base == 0 {
                *base = 36;
                message = format!("Base set to Z+1.");
            } else {
                *base = new_base;
                message = format!("Base set to {}.", (digit as char).to_uppercase());
            }
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
            message = format!("Precision set to {} digits.", value);
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
        _ => return Err((format!("Unknown command!"), index)),
    };

    Err((message, std::usize::MAX))
}
fn evaluate_tokens(
    tokens: &[Token],
    base: u8,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
) -> Complex {
    let mut output_queue: Vec<Complex> = Vec::new();
    let mut operator_stack: Vec<char> = Vec::new();

    for token in tokens {
        if token.operator == '\0' {
            // It's a number, push it to the output queue
            output_queue.push(token2num(token, base, precision));
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
                        );
                    }
                    if operator_stack.pop() != Some('(') {
                        panic!("Mismatched parentheses");
                    }
                }
                'p' => output_queue.push(Complex::with_val(
                    precision,
                    rug::Float::with_val(precision, rug::float::Constant::Pi),
                )),
                'E' => output_queue.push(Complex::with_val(
                    precision,
                    rug::Float::with_val(precision, rug::float::Constant::Euler),
                )),
                'r' => output_queue.push(generate_random(precision, rand_state)),
                _ => {
                    while !operator_stack.is_empty()
                        && get_precedence(*operator_stack.last().unwrap())
                            >= get_precedence(token.operator)
                        && *operator_stack.last().unwrap() != '('
                    {
                        apply_operator(
                            &mut output_queue,
                            operator_stack.pop().unwrap(),
                            precision,
                            rand_state,
                            base,
                        );
                    }
                    operator_stack.push(token.operator);
                }
            }
        }
    }

    // Apply any remaining operators
    while let Some(op) = operator_stack.pop() {
        if op == '(' || op == ')' {
            panic!("Mismatched parentheses");
        }
        apply_operator(&mut output_queue, op, precision, rand_state, base);
    }

    output_queue
        .pop()
        .unwrap_or_else(|| Complex::with_val(precision, 0))
}
fn apply_operator(
    output_queue: &mut Vec<Complex>,
    op: char,
    precision: u32,
    rand_state: &mut rug::rand::RandState,
    base: u8,
) {
    match op {
        'n' => {
            // Unary negation
            if let Some(operand) = output_queue.pop() {
                output_queue.push(-operand);
            }
        }
        'a' | 'C' | 'S' | 'T' | 'c' | 'i' | 'l' | 'L' | 'r' | 'e' | 's' | 'q' | 't' => {
            if let Some(operand) = output_queue.pop() {
                let result = match op {
                    'a' => operand.abs(),
                    'C' => operand.acos(),
                    'S' => operand.asin(),
                    'T' => operand.atan(),
                    'c' => operand.cos(),
                    'i' => Complex::with_val(precision, (operand.imag(), 0)),
                    'l' => operand.ln(),
                    'L' => operand.ln() / Float::with_val(precision, base).ln(),
                    'r' => generate_random(precision, rand_state),
                    'e' => Complex::with_val(precision, (operand.real(), 0)),
                    's' => operand.sin(),
                    'q' => operand.sqrt(),
                    't' => operand.tan(),
                    _ => panic!("Unknown operator!"),
                };
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
                    _ => panic!("Unknown operator!"),
                };
                output_queue.push(result);
            }
        }
    }
}
fn get_precedence(op: char) -> Precedence {
    match op {
        '+' | '-' => Precedence::Addition,
        '*' | '/' | '%' => Precedence::Multiplication,
        '^' => Precedence::Exponentiation,
        'a' | 'C' | 'S' | 'T' | 'c' | 'i' | 'l' | 'L' | 'r' | 'e' | 's' | 'q' | 't' => {
            Precedence::Function
        }
        'n' => Precedence::UnaryMinus,
        _ => Precedence::Lowest,
    }
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
