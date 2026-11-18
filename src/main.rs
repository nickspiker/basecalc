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

// AI notes:
// 1. Key Pain Points Observed:
// - Constantly needing to copy/paste long number strings
// - Easy to make transcription errors
// - Can't easily verify steps against expected values
// - No way to track transformations systematically
//
// 2. Features Needed:
// - Variables/stack for intermediate values
// - Way to mark verification points/assertions
// - Record/replay sequences of operations
// - Step-by-step comparison with reference implementations
// - Debug mode to show precision loss at each step, encoding patterns and set precision/base
//
// 3. Specific Additions:
// - Store/name intermediate calculations, present formatted command and output to user
// - Compare results within epsilon
// - Track precision loss thru operation chains
// - Save/load common transformation sequences
// - Built-in coordinate system transforms, int/float conversions, etc.
// - All common operators should be defined and integrated
//
// 4. Syntax Suggestions:
// - Store value: -> var_name
// - Assert within epsilon: =~ expected_value
// - Show precision loss: .precision
// - Save sequence: .save transform_name

use basecalc::*;
use colored::*;
use dirs;
use std::fs;
use std::io::{self, Write};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use vsf::vsf::*;
fn main() -> rustyline::Result<()> {
    let mut state = match load_state() {
        Some(s) => {
            // Initialize DEBUG atomic boolean from loaded state
            DEBUG.store(s.debug, Ordering::Relaxed);
            debug_println(&format!(
                "Loaded state: Base: {}, Digits: {}, Radians: {}, History: {} entries, Debug: {}",
                s.base,
                s.digits,
                s.radians,
                s.history.len(),
                s.debug
            ));
            for (i, entry) in s.history.iter().enumerate() {
                debug_println(&format!("Loaded history entry {}: {}", i, entry));
            }
            s
        }
        None => {
            debug_println("Using default state");
            BasecalcState::new()
        }
    };

    print_stylized_intro(&state.colours);
    println!();
    print_settings(&state);

    loop {
        let entry = terminal_line_entry(&mut state);
        println!();
        match entry {
            Ok(Some(line)) => {
                debug_println(&format!("Processing input: '{}'", line));
                match tokenize(&line, &mut state) {
                    Ok(tokens) => {
                        match evaluate_tokens(&tokens, &mut state) {
                            Ok(result) => {
                                let result_vec = paint(&result_spans(&result, &state), &state.colours);
                                state.prev_result = result.value;
                                for coloured_string in result_vec {
                                    print!("{}", coloured_string);
                                }
                                println!();
                            }
                            Err(err) => println!(
                                "{}",
                                err.truecolor(state.colours.error.0, state.colours.error.1, state.colours.error.2)
                            ),
                        }

                        debug_println(&format!("Added to history: {}", line));
                    }
                    Err(Halt::Wrong(msg, pos)) => {
                        println!(
                            "  {}{}",
                            " ".repeat(pos),
                            "^".truecolor(
                                state.colours.carat.0,
                                state.colours.carat.1,
                                state.colours.carat.2
                            )
                        );
                        println!(
                            "{}",
                            msg.truecolor(
                                state.colours.error.0,
                                state.colours.error.1,
                                state.colours.error.2
                            )
                        );
                    }
                    Err(Halt::Help) => {
                        for line in get_help_text(&state) {
                            print!("{}", line);
                        }
                        println!("\n");
                        print_settings(&state);
                        println!();
                    }
                    Err(Halt::Tested(outcomes)) => {
                        print_tests(&outcomes, &state.colours);
                        print_message(&Halt::Tested(outcomes).message(), &state.colours);
                    }
                    Err(Halt::Dms(spans)) => {
                        for block in paint(&spans, &state.colours) {
                            print!("{}", block);
                        }
                        println!();
                    }
                    Err(Halt::Said(msg)) => print_message(&msg, &state.colours),
                }
                // Save state after each entry
                state.debug = DEBUG.load(Ordering::Relaxed);
                if let Err(e) = save_state(&state) {
                    eprintln!("Failed to save state: {}", e);
                }
            }
            Ok(None) => {
                println!("Goodbye!");
                break;
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

fn print_message(msg: &str, colours: &RGBValues) {
    println!("{}", msg.truecolor(colours.message.0, colours.message.1, colours.message.2));
}

/// The engine's spans in the terminal's colours.
fn paint(spans: &[Span], colours: &RGBValues) -> Vec<ColoredString> {
    spans
        .iter()
        .map(|s| {
            let c = match s.role {
                Role::Plain => return s.text.as_str().normal(),
                Role::LoneInteger => colours.lone_integer,
                Role::LoneFraction => colours.lone_fraction,
                Role::RealInteger => colours.real_integer,
                Role::RealFraction => colours.real_fraction,
                Role::ImaginaryInteger => colours.imaginary_integer,
                Role::ImaginaryFraction => colours.imaginary_fraction,
                Role::Exponent => colours.exponent,
                Role::Decimal => colours.decimal,
                Role::Sign => colours.sign,
                Role::Tilde => colours.tilde,
                Role::Carat => colours.carat,
                Role::Error => colours.error,
                Role::Brackets => colours.brackets,
                Role::Comma => colours.comma,
                Role::Colon => colours.colon,
                Role::Nan => colours.nan,
                Role::Message => colours.message,
            };
            s.text.as_str().truecolor(c.0, c.1, c.2)
        })
        .collect()
}

/// The self test as the terminal shows it: each line, what it showed, and pass or fail.
fn print_tests(outcomes: &[TestOutcome], colours: &RGBValues) {
    for o in outcomes {
        println!("> {}", o.input);
        for coloured_string in paint(&o.shown, colours) {
            print!("{}", coloured_string);
        }
        println!();
        if o.passed() {
            println!("{}", "Pass!".green());
        } else {
            println!("{}", "fail!".red());
            println!("Sposta: '{}'", o.expected);
            println!("Gots  : '{}'", o.got());
        }
        println!();
    }
}

fn terminal_line_entry(state: &mut BasecalcState) -> io::Result<Option<String>> {
    let mut stdout = io::stdout().into_raw_mode()?;
    let stdin = io::stdin();
    let mut chars = stdin.keys();
    let mut user_input = String::new();
    let mut cursor_position = 0;

    loop {
        // Ensure cursor_position is within bounds
        cursor_position = cursor_position.min(state.current_entry.len());

        write!(
            stdout,
            "\r\x1B[2K> {}{}",
            &state.current_entry[..cursor_position],
            &state.current_entry[cursor_position..]
        )?;
        write!(stdout, "\r\x1B[{}C", cursor_position + 2)?; // +2 for "> "
        stdout.flush()?;

        if let Some(Ok(key)) = chars.next() {
            match key {
                Key::Left => {
                    if cursor_position > 0 {
                        cursor_position -= 1;
                    }
                }
                Key::Right => {
                    if cursor_position < state.current_entry.len() {
                        cursor_position += 1;
                    }
                }
                Key::Up => {
                    if state.history_index < state.history.len() {
                        state.history_index += 1;
                        let index = state.history.len() - state.history_index;
                        state.current_entry = state.history[index].clone();
                        cursor_position = state.current_entry.len();
                    }
                }
                Key::Down => {
                    if state.history_index > 0 {
                        state.history_index -= 1;
                        if state.history_index == 0 {
                            state.current_entry = user_input.clone();
                        } else {
                            let index = state.history.len() - state.history_index;
                            state.current_entry = state.history[index].clone();
                        }
                        cursor_position = state.current_entry.len();
                    }
                }
                Key::Char('\n') => {
                    if state.current_entry.is_empty() {
                        return Ok(None);
                    }
                    let entry = state.current_entry.clone();
                    state.history.push(entry.clone());
                    state.current_entry.clear();
                    user_input.clear();
                    state.history_index = 0;
                    writeln!(stdout)?;
                    return Ok(Some(entry));
                }
                Key::Char(c) => {
                    state.current_entry.insert(cursor_position, c);
                    cursor_position += 1;
                }
                Key::Backspace => {
                    if cursor_position > 0 {
                        state.current_entry.remove(cursor_position - 1);
                        cursor_position -= 1;
                    }
                }
                Key::Delete => {
                    if cursor_position < state.current_entry.len() {
                        state.current_entry.remove(cursor_position);
                    }
                }
                Key::Ctrl('c') => {
                    writeln!(stdout, "\nInterrupted")?;
                    return Ok(None);
                }
                _ => {}
            }
        }
    }
}

fn get_state_file_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("basecalc");
    fs::create_dir_all(&path).expect("Failed to create config directory");
    path.push("state.vsf");
    path
}
fn save_state(state: &BasecalcState) -> std::io::Result<()> {
    let path = get_state_file_path();
    let temp_path = path.with_extension("vsf-");

    let vsf_data = create_vsf_data(state)?;

    let mut file = fs::File::create(&temp_path)?;
    file.write_all(&vsf_data)?;
    file.sync_all()?;

    fs::rename(temp_path, path)?;
    Ok(())
}
fn load_state() -> Option<BasecalcState> {
    let path = get_state_file_path();
    debug_println(&mut format!("Attempting to load state from: {:?}", path));

    if path.exists() {
        match fs::read(&path) {
            Ok(data) => {
                debug_println("File read successfully");
                let mut pointer = 0;
                match parse_vsf(&data, &mut pointer) {
                    Ok(state) => {
                        // Update the DEBUG atomic boolean
                        DEBUG.store(state.debug, Ordering::Relaxed);
                        debug_println(&format!("Debug mode set to: {}", state.debug));

                        debug_println("State parsed successfully");
                        Some(state)
                    }
                    Err(e) => {
                        eprintln!("Error parsing state file: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading state file: {}", e);
                None
            }
        }
    } else {
        debug_println("State file does not exist");
        None
    }
}
fn parse_vsf(data: &[u8], pointer: &mut usize) -> Result<BasecalcState, std::io::Error> {
    debug_println(&format!("Starting VSF parsing"));

    // Check magic number
    if data.len() < 4 || &data[0..3] != b"R\xC3\x85" {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Magic number does not match 'RÅ' at decimal offset {} bytes",
                *pointer
            ),
        ));
    }
    *pointer = 3;
    debug_println(&format!("Magic number 'RÅ' verified"));

    // Check for opening angle bracket
    if data[*pointer] != b'<' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Expected header opening '<' after magic number at decimal offset {} bytes",
                *pointer
            ),
        ));
    }
    *pointer += 1;
    debug_println(&format!("Opening angle bracket '<' found"));

    // Parse header length
    let header_length = parse(data, pointer)?;
    let header_length_bytes;
    if let VsfType::b(length) = header_length {
        if length % 8 != 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Header length is not a multiple of 8 at decimal offset {} bytes",
                    *pointer
                ),
            ));
        }
        header_length_bytes = length / 8;
        debug_println(&format!(
            "Header length: {} bits ({} bytes)",
            length, header_length_bytes
        ));
    } else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Expected header length of type 'b' at decimal offset {} bytes",
                *pointer
            ),
        ));
    }

    // Parse version and backward version
    let first = parse(data, pointer)?;
    let second = parse(data, pointer)?;

    let (_version, backward_version) = match (&first, &second) {
        (VsfType::z(v), VsfType::y(bv)) => {
            debug_println(&format!("Version: {}, Backward version: {}", v, bv));
            (*v, *bv)
        }
        (VsfType::y(bv), VsfType::z(v)) => {
            debug_println(&format!("Version: {}, Backward version: {}", v, bv));
            (*v, *bv)
        }
        _ => {
            return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Expected version (z) and backward version (y) at decimal offset {} bytes, found {:?} and {:?}",
                *pointer, first, second
            ),
        ));
        }
    };

    if backward_version > 1 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unsupported backward version {}!", backward_version),
        ));
    }

    // Parse label definition count
    let label_count_vsf = parse(data, pointer)?;
    let label_count;
    if let VsfType::c(count) = label_count_vsf {
        label_count = count;
        debug_println(&format!("Label count: {}", label_count));
    } else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Expected label count 'c' at decimal offset {} bytes",
                *pointer
            ),
        ));
    }

    let mut basecalc_offset = 0;
    let mut basecalc_size = 0;
    let mut basecalc_count = 0;

    // Parse label definitions
    debug_println(&format!("Parsing label definitions"));
    for i in 0..label_count {
        debug_println(&format!(
            "Parsing label definition {}/{}",
            i + 1,
            label_count
        ));
        if data[*pointer] != b'(' {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected label set definition '(' at decimal offset {} bytes",
                    *pointer
                ),
            ));
        }
        *pointer += 1;

        if let VsfType::d(label_str) = parse(data, pointer)? {
            debug_println(&format!("Found label: {}", label_str));
            if label_str == "basecalc state" {
                let mut offset = None;
                let mut size = None;
                let mut count = None;

                // Parse offset, size, and count in any order
                while data[*pointer] != b')' {
                    match parse(data, pointer)? {
                        VsfType::o(o) => {
                            debug_println(&format!("basecalc state offset: {}", o));
                            offset = Some(o);
                        }
                        VsfType::b(s) => {
                            debug_println(&format!("basecalc state size: {}", s));
                            size = Some(s);
                        }
                        VsfType::c(c) => {
                            debug_println(&format!("basecalc state count: {}", c));
                            count = Some(c);
                        }
                        _ => {
                            debug_println(&format!(
                                "Ignoring unknown type for future compatibility"
                            ));
                        }
                    }
                }

                basecalc_offset = offset.ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "Missing offset for basecalc state")
                })?;
                basecalc_size = size.ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "Missing size for basecalc state")
                })?;
                basecalc_count = count.ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "Missing count for basecalc state")
                })?;
            } else {
                debug_println(&format!("Skipping unknown label: {}", label_str));
                // Skip other label definitions
                while data[*pointer] != b')' {
                    parse(data, pointer)?;
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Expected label 'd' at decimal offset {} bytes", *pointer),
            ));
        }

        if data[*pointer] != b')' {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected ')' at end of label definition at decimal offset {} bytes",
                    *pointer
                ),
            ));
        }
        *pointer += 1;
    }

    // Check for closing angle bracket
    if data[*pointer] != b'>' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Expected header closing '>' at decimal offset {} bytes",
                *pointer
            ),
        ));
    }
    *pointer += 1;
    debug_println(&format!("Header closing '>' found"));

    if *pointer != header_length_bytes {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Header length mismatch: expected {} bytes, got {} bytes",
                header_length_bytes, pointer
            ),
        ));
    }

    // Initialize basecalc state with default values
    let mut base = 0;
    let mut digits = 0;
    let mut radians_flag: u8 = 3; // 3 indicates missing value
    let mut history = Vec::new();
    let mut debug_flag = false;

    let mut history_offset;
    let mut history_size;
    let mut history_count;

    // Parse basecalc state if found
    if basecalc_offset > 0 && basecalc_size > 0 && basecalc_count > 0 {
        debug_println(&format!("Parsing basecalc state"));
        // Move pointer to basecalc state data
        *pointer = (basecalc_offset / 8) as usize;
        debug_println(&format!(
            "Moved pointer to basecalc state data at offset: {}",
            *pointer
        ));

        // Parse label set
        if data[*pointer] != b'[' {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected '[' for label set at decimal offset {} bytes",
                    *pointer
                ),
            ));
        }
        *pointer += 1;

        for i in 0..basecalc_count {
            debug_println(&format!(
                "Parsing basecalc state label {}/{}",
                i + 1,
                basecalc_count
            ));
            if data[*pointer] != b'(' {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Expected '(' for label at decimal offset {} bytes",
                        *pointer
                    ),
                ));
            }
            *pointer += 1;

            let label = parse(data, pointer)?;
            if let VsfType::d(label_str) = label {
                debug_println(&format!("Found basecalc state label: {}", label_str));
                match label_str.as_str() {
                    "base" => {
                        if data[*pointer] != b':' {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected ':' after 'base' label at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                        *pointer += 1;
                        if let VsfType::u3(value) = parse(data, pointer)? {
                            base = value;
                            debug_println(&format!("Parsed base: {}", base));
                        } else {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected u3 type for 'base' at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                    }
                    "digits" => {
                        if data[*pointer] != b':' {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected ':' after 'digits' label at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                        *pointer += 1;
                        match parse(data, pointer)? {
                            VsfType::u(value) => {
                                digits = value as usize;
                            }
                            VsfType::u3(value) => {
                                digits = value as usize;
                            }
                            VsfType::u4(value) => {
                                digits = value as usize;
                            }
                            VsfType::u5(value) => {
                                digits = value as usize;
                            }
                            VsfType::u6(value) => {
                                digits = value as usize;
                            }
                            VsfType::u7(value) => {
                                digits = value as usize;
                            }
                            _ => {
                                return Err(Error::new(
                                    ErrorKind::InvalidData,
                                    format!(
                                        "Expected u type for 'digits' at decimal offset {} bytes",
                                        *pointer
                                    ),
                                ));
                            }
                        }
                        debug_println(&format!("Parsed digits: {}", digits));
                    }
                    "radians" => {
                        if data[*pointer] != b':' {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected ':' after 'radians' label at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                        *pointer += 1;
                        let a = parse(data, pointer);
                        if let VsfType::u0(value) = a? {
                            radians_flag = if value { 1 } else { 0 };
                            debug_println(&format!("Parsed radians: {}", radians_flag));
                        } else {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected u0 type for 'radians' at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                    }
                    "history" => {
                        let mut offset = None;
                        let mut size = None;
                        let mut count = None;

                        if data[*pointer] != b':' {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected ':' after 'history' label at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                        *pointer += 1;

                        // Parse offset, size, and count in any order
                        while data[*pointer] != b')' {
                            match parse(data, pointer)? {
                                VsfType::o(o) => {
                                    debug_println(&format!("basecalc history offset: {}", o / 8));
                                    offset = Some(o);
                                }
                                VsfType::b(s) => {
                                    debug_println(&format!("basecalc history size: {}", s / 8));
                                    size = Some(s);
                                }
                                VsfType::c(c) => {
                                    debug_println(&format!("basecalc history count: {}", c));
                                    count = Some(c);
                                }
                                _ => {
                                    debug_println(&format!(
                                        "Ignoring unknown type for future compatibility"
                                    ));
                                }
                            }
                        }

                        history_offset = offset.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                "Missing offset for basecalc history",
                            )
                        })?;
                        history_size = size.ok_or_else(|| {
                            Error::new(ErrorKind::InvalidData, "Missing size for basecalc history")
                        })?;
                        history_count = count.ok_or_else(|| {
                            Error::new(ErrorKind::InvalidData, "Missing count for basecalc history")
                        })?;

                        let mut history_pointer = (history_offset / 8) as usize;
                        debug_println(&format!(
                            "Moved pointer to basecalc history data at offset: {}",
                            history_pointer
                        ));

                        // Parse history entries
                        for entry in 0..history_count {
                            debug_println(&format!(
                                "Parsing basecalc history entry {}/{}",
                                entry + 1,
                                history_count
                            ));
                            match parse(data, &mut history_pointer)? {
                                VsfType::x(mut entry) => {
                                    if entry.ends_with('\n') {
                                        entry.truncate(entry.len() - 1);
                                    } else {
                                        return Err(Error::new(
                                            ErrorKind::InvalidData,
                                            format!(
                                                "Expected newline at end of history entry at decimal offset {} bytes",
                                                history_pointer
                                            ),
                                        ));
                                    }
                                    debug_println(&format!("Parsed history entry: {}", entry));
                                    history.push(entry);
                                }
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::InvalidData,
                                        format!(
                                            "Expected x type for history entry at decimal offset {} bytes",
                                            history_pointer
                                        ),
                                    ));
                                }
                            }
                        }
                        if history_pointer != (history_offset + history_size) / 8 {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "History length mismatch: expected {} bytes, got {} bytes",
                                    history_size, history_pointer
                                ),
                            ));
                        }
                    }
                    "DEBUG" => {
                        if data[*pointer] != b':' {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected ':' after 'DEBUG' label at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                        *pointer += 1;
                        let a = parse(data, pointer);
                        if let VsfType::u0(value) = a? {
                            debug_flag = value;
                            debug_println(&format!("Parsed DEBUG: {}", debug_flag));
                        } else {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Expected u0 type (boolean) for 'DEBUG' at decimal offset {} bytes",
                                    *pointer
                                ),
                            ));
                        }
                    }
                    _ => {
                        debug_println(&format!(
                            "Skipping unknown basecalc state label: {}",
                            label_str
                        ));
                        // Skip unknown labels
                        while data[*pointer] != b')' {
                            if data[*pointer] == b':' {
                                *pointer += 1;
                            } else {
                                parse(data, pointer)?;
                            }
                        }
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Expected label of type 'd' at decimal offset {} bytes",
                        *pointer
                    ),
                ));
            }

            if data[*pointer] != b')' {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Expected ')' after label value at decimal offset {} bytes",
                        *pointer
                    ),
                ));
            }
            *pointer += 1;
        }

        if data[*pointer] != b']' {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Expected ']' at end of label set at decimal offset {} bytes",
                    *pointer
                ),
            ));
        }
        *pointer += 1;
        debug_println(&format!("Finished parsing basecalc state"));
    } else {
        debug_println(&format!("No basecalc state found in the file"));
    }

    // Check if we got valid data
    debug_println(&format!("Checking validity of parsed data"));
    if base == 0 || digits == 0 || radians_flag == 3 || history.is_empty() {
        if base == 0 {
            debug_println(&format!("Error: Missing base"));
            return Err(Error::new(ErrorKind::InvalidData, "Missing base"));
        }
        if digits == 0 {
            debug_println(&format!("Error: Missing digits"));
            return Err(Error::new(ErrorKind::InvalidData, "Missing digits"));
        }
        if radians_flag == 3 {
            debug_println(&format!("Error: Missing radians flag"));
            return Err(Error::new(ErrorKind::InvalidData, "Missing radians"));
        }
        if history.is_empty() {
            debug_println(&format!("Error: Missing history"));
            return Err(Error::new(ErrorKind::InvalidData, "Missing history"));
        }
    }

    let radians = radians_flag == 1;
    debug_println(&format!("Final parsed values:"));
    debug_println(&format!("  Base: {}", base));
    debug_println(&format!("  Digits: {}", digits));
    debug_println(&format!("  Radians: {}", radians));
    debug_println(&format!("  History entries: {}", history.len()));

    debug_println(&format!("VSF parsing completed successfully"));
    let mut state = BasecalcState::new();
    state.base = base;
    state.digits = digits;
    state.set_precision();
    state.radians = radians;
    state.history = history;
    state.debug = debug_flag;
    Ok(state)
}
fn create_vsf_data(basecalc_state: &BasecalcState) -> Result<Vec<u8>, std::io::Error> {
    let mut history_entries_combined = Vec::new();
    for entry in &basecalc_state.history {
        let entry_with_return = entry.clone() + "\n";
        history_entries_combined.append(&mut VsfType::x(entry_with_return).flatten()?);
    }
    let mut vsf = vec!["RÅ".as_bytes().to_owned()];

    // Header
    let mut header_index = 0;
    vsf[header_index].append(&mut b"<".to_vec());
    let header_length_index = vsf.len();
    let mut header_length = 42;
    vsf.push(VsfType::b(header_length).flatten()?); // Placeholder for header length in bits, always first
    header_index = vsf.len();
    vsf.push(VsfType::z(1).flatten()?); // Version
    vsf[header_index].append(&mut VsfType::y(1).flatten()?); // Backward version
    vsf[header_index].append(&mut VsfType::c(1).flatten()?); // label definition count
    vsf[header_index].append(&mut b"(".to_vec()); // Start of label definition
    vsf[header_index].append(&mut VsfType::d("basecalc state".to_string()).flatten()?); // VsfType d for the data type
    let label_offset_index = vsf.len();
    let mut label_offset = 42;
    vsf.push(VsfType::o(label_offset).flatten()?); // Placeholder for offset to basecalc state
    let label_size_index = vsf.len();
    let mut label_size = 42;
    vsf.push(VsfType::b(label_size).flatten()?); // Placeholder for size of basecalc state
    header_index = vsf.len();
    vsf.push(VsfType::c(5).flatten()?); // Number of elements in basecalc state
    vsf[header_index].append(&mut b")".to_vec());
    vsf[header_index].append(&mut b">".to_vec());
    let header_end_index = vsf.len();

    // Label set
    header_index = vsf.len();
    vsf.push(b"[".to_vec());
    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("base".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    vsf[header_index].append(&mut VsfType::u3(basecalc_state.base).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("digits".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    vsf[header_index].append(&mut VsfType::u(basecalc_state.digits).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("radians".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    vsf[header_index].append(&mut VsfType::u0(basecalc_state.radians).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("history".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    let history_offset_index = vsf.len();
    let mut history_offset = 42;
    vsf.push(VsfType::o(history_offset).flatten()?);
    header_index = vsf.len();
    vsf.push(VsfType::b(history_entries_combined.len() * 8).flatten()?);
    vsf[header_index].append(&mut VsfType::c(basecalc_state.history.len()).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("DEBUG".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    vsf[header_index].append(&mut VsfType::u0(basecalc_state.debug).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"]".to_vec());

    let mut prev_header_length = 0;
    let mut prev_label_offset = 0;
    let mut prev_label_size = 0;
    let mut prev_history_offset = 0;

    while header_length != prev_header_length
        || label_offset != prev_label_offset
        || label_size != prev_label_size
        || history_offset != prev_history_offset
    {
        prev_header_length = header_length;
        prev_label_offset = label_offset;
        prev_label_size = label_size;
        prev_history_offset = history_offset;

        header_length = 0;
        for i in 0..header_end_index {
            header_length += vsf[i].len();
        }
        vsf[header_length_index] = VsfType::b(header_length * 8).flatten()?;

        label_offset = header_length;
        vsf[label_offset_index] = VsfType::o(label_offset * 8).flatten()?;

        label_size = 0;
        for i in header_end_index..vsf.len() {
            let mut vsfi = "".to_owned();
            for index in 0..vsf[i].len() {
                let id = vsf[i][index];
                if id >= 32 && id <= 126 {
                    vsfi.push(id as char);
                } else {
                    vsfi.push(' ');
                }
            }
            label_size += vsf[i].len();
        }
        vsf[label_size_index] = VsfType::b(label_size * 8).flatten()?;

        history_offset = label_offset + label_size;
        vsf[history_offset_index] = VsfType::o(history_offset * 8).flatten()?;
    }

    vsf.push(history_entries_combined);

    let vsf_vector: Vec<u8> = vsf.into_iter().flatten().collect();
    if DEBUG.load(Ordering::Relaxed) {
        print_colorized_vsf(&vsf_vector);
    }
    Ok(vsf_vector)
}
fn print_colorized_vsf(vsf_data: &[u8]) {
    let mut first_line = String::new();
    let mut second_line = String::new();

    for &byte in vsf_data {
        if is_keyboard_printable(byte) {
            first_line.push_str(&format!("{}", (byte as char).to_string().green()));
            second_line.push(' ');
        } else {
            let hex = format!("{:02X}", byte).as_bytes().to_owned();
            first_line.push_str(&format!("{}", (hex[0] as char).to_string().red()));
            second_line.push_str(&format!("{}", (hex[1] as char).to_string().red()));
        }
    }
    let mut index_lines = Vec::new();
    for line_count in 0..(vsf_data.len() as f64).log10().floor() as usize + 1 {
        let mut line = String::new();
        for i in 0..vsf_data.len() {
            let i_trunc = i / (10usize).pow(line_count as u32);
            if i_trunc > 0 {
                line.push_str(&format!("{}", i_trunc % 10));
            } else {
                line.push(' ');
            }
        }
        index_lines.push(line.blue());
    }

    println!("{}", second_line);
    println!("{}", first_line);
    for line in index_lines {
        println!("{}", line);
    }
}
fn is_keyboard_printable(byte: u8) -> bool {
    match byte {
        32..=126 => true, // Printable ASCII characters (including space)
        _ => false,
    }
}
fn print_settings(state: &BasecalcState) {
    print!(
        "{}",
        "Currently ".truecolor(
            state.colours.real_integer.0,
            state.colours.real_integer.1,
            state.colours.real_integer.2
        )
    );
    print!(
        "{}",
        "Base: ".truecolor(
            state.colours.lone_integer.0,
            state.colours.lone_integer.1,
            state.colours.lone_integer.2
        )
    );
    let base_char = if state.base < 10 {
        (state.base + b'0') as char
    } else {
        (state.base - 10 + b'A') as char
    };
    print!(
        "{}",
        base_char.to_string().truecolor(
            state.colours.lone_fraction.0,
            state.colours.lone_fraction.1,
            state.colours.lone_fraction.2
        )
    );
    print!(
        " ({})",
        get_base_name(state.base).unwrap().truecolor(
            state.colours.lone_fraction.0,
            state.colours.lone_fraction.1,
            state.colours.lone_fraction.2
        )
    );
    print!(
        "{}",
        ", Digits: ".truecolor(
            state.colours.lone_integer.0,
            state.colours.lone_integer.1,
            state.colours.lone_integer.2
        )
    );
    print!(
        "{}",
        format_int(state.digits, state.base as usize).truecolor(
            state.colours.lone_fraction.0,
            state.colours.lone_fraction.1,
            state.colours.lone_fraction.2
        )
    );
    print!(
        "{}",
        ", Trig units: ".truecolor(
            state.colours.lone_integer.0,
            state.colours.lone_integer.1,
            state.colours.lone_integer.2
        )
    );
    println!(
        "{}",
        if state.radians {
            "radians".truecolor(
                state.colours.lone_fraction.0,
                state.colours.lone_fraction.1,
                state.colours.lone_fraction.2,
            )
        } else {
            "degrees".truecolor(
                state.colours.lone_fraction.0,
                state.colours.lone_fraction.1,
                state.colours.lone_fraction.2,
            )
        }
    );
}
fn print_stylized_intro(colours: &RGBValues) {
    let ascii_art = r#"
 _                              _      
| |                            | |     
| |__   __ _ ___  ___  ___ __ _| | ___ 
| '_ \ / _` / __|/ _ \/ __/ _` | |/ __|
| |_) | (_| \__ \  __/ (_| (_| | | (__ 
|_.__/ \__,_|___/\___|\___\__,_|_|\___|   
"#;

    println!(
        "{}",
        ascii_art.truecolor(colours.brackets.0, colours.brackets.1, colours.brackets.2)
    );

    println!(
        "{}",
        "Welcome to Basecalc!"
            .truecolor(colours.decimal.0, colours.decimal.1, colours.decimal.2)
            .bold()
    );

    println!(
        "\n{}",
        "Your gateway to mathematical adventures!"
            .truecolor(
                colours.lone_fraction.0,
                colours.lone_fraction.1,
                colours.lone_fraction.2
            )
            .italic()
    );

    println!(
        "\n{}",
        "For help, simply type:".truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2
        )
    );

    println!(
        "{}",
        ":help"
            .truecolor(colours.exponent.0, colours.exponent.1, colours.exponent.2)
            .bold()
    );

    println!(
        "{}",
        "Then press 'Enter'!".truecolor(
            colours.lone_integer.0,
            colours.lone_integer.1,
            colours.lone_integer.2
        )
    );

    println!(
        "\n{}",
        "Happy calculating!"
            .truecolor(colours.message.0, colours.message.1, colours.message.2)
            .bold()
    );
}
fn get_help_text(global_state: &BasecalcState) -> Vec<ColoredString> {
    let mut local_state = global_state.clone();
    let mut help_text: Vec<ColoredString> = Vec::new();

    // Geeky Intro
    help_text.push("Welcome to basecalc!\n".truecolor(
        local_state.colours.decimal.0,
        local_state.colours.decimal.1,
        local_state.colours.decimal.2,
    ));
    help_text.push("
Greetings, intrepid mathematical explorer!  This isn't just any ordinary number-crunching gizmo - it's your towel in the cosmos!

Whether you're calculating the odds of successfully navigating an asteroid field, determining the exact amount of Pangalactic Gargleblasters needed for a party of trans-dimensional beings, or just trying to split the bill at the Restaurant at the End of the Universe, basecalc has got you covered!

Remember, DON'T PANIC! With basecalc, you're always just a few keystrokes away from mathematical enlightenment. So grab your towel, keep your wits about you, and prepare to compute where no one has computed before!
".normal());

    // Commands
    help_text.push("\nCommands:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    let commands = [
        (
            ":base ",
            "<digit>  ",
            "Set number base (2 to Z+1, 0 for Z+1)",
        ),
        (":digits ", "<value>", "Adjust display precision"),
        (
            ":radians       ",
            "",
            "Switch to radians (for the cool kids)",
        ),
        (":degrees       ", "", "Switch to degrees (if you must)"),
        (":help          ", "", "You're looking at it!"),
        (":debug         ", "", "Toggle inspection mode"),
        (":test          ", "", "Ensure calculator isn't a lemon"),
    ];

    for (cmd, alt, desc) in commands.iter() {
        help_text.push(format!("  {}", cmd).truecolor(
            local_state.colours.lone_integer.0,
            local_state.colours.lone_integer.1,
            local_state.colours.lone_integer.2,
        ));
        help_text.push(alt.truecolor(
            local_state.colours.nan.0,
            local_state.colours.nan.1,
            local_state.colours.nan.2,
        ));
        help_text.push(format!(" - {}\n", desc).truecolor(
            local_state.colours.lone_fraction.0,
            local_state.colours.lone_fraction.1,
            local_state.colours.lone_fraction.2,
        ));
    }

    // Constants
    help_text.push("\nConstants:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    for &(name, symbol, description) in CONSTANTS.iter() {
        let token = Token {
            operator: symbol,
            ..Token::new()
        };
        let value = token2num(&token, &mut local_state);
        let value_string = paint(&num2string(&value, &local_state), &local_state.colours);

        help_text.push(format!("  {:<7}", name).truecolor(
            local_state.colours.lone_integer.0,
            local_state.colours.lone_integer.1,
            local_state.colours.lone_integer.2,
        ));
        help_text.push(format!("- {} ", description).truecolor(
            local_state.colours.lone_fraction.0,
            local_state.colours.lone_fraction.1,
            local_state.colours.lone_fraction.2,
        ));
        for part in value_string {
            help_text.push(part);
        }
        help_text.push("\n".truecolor(
            local_state.colours.brackets.0,
            local_state.colours.brackets.1,
            local_state.colours.brackets.2,
        ));
    }

    // Operators and Functions
    help_text.push("\nUnary Operators:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    for &(name, _, operands, description) in OPERATORS.iter() {
        if operands == 1 && name != "(" && name != ")" {
            help_text.push(format!("  {:<8}", name).truecolor(
                local_state.colours.lone_integer.0,
                local_state.colours.lone_integer.1,
                local_state.colours.lone_integer.2,
            ));
            let capitalized_description = description[0..1].to_uppercase() + &description[1..];
            help_text.push(format!("- {}\n", capitalized_description).truecolor(
                local_state.colours.lone_fraction.0,
                local_state.colours.lone_fraction.1,
                local_state.colours.lone_fraction.2,
            ));
        }
    }

    help_text.push("\nBinary Operators:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    for &(name, _, operands, description) in OPERATORS.iter() {
        if operands == 2 {
            help_text.push(format!("  {:<7}", name).truecolor(
                local_state.colours.lone_integer.0,
                local_state.colours.lone_integer.1,
                local_state.colours.lone_integer.2,
            ));
            let capitalized_description = description[0..1].to_uppercase() + &description[1..];
            help_text.push(format!("- {}\n", capitalized_description).truecolor(
                local_state.colours.lone_fraction.0,
                local_state.colours.lone_fraction.1,
                local_state.colours.lone_fraction.2,
            ));
        }
    }

    // Grouping
    help_text.push("\nGrouping:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    help_text.push("  ( )   ".truecolor(
        local_state.colours.lone_integer.0,
        local_state.colours.lone_integer.1,
        local_state.colours.lone_integer.2,
    ));
    help_text.push("- Parentheses for grouping expressions\n".truecolor(
        local_state.colours.lone_fraction.0,
        local_state.colours.lone_fraction.1,
        local_state.colours.lone_fraction.2,
    ));

    // Variable assignment and usage
    help_text.push("\nVariables:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    help_text.push("  @name=value  ".truecolor(
        local_state.colours.lone_integer.0,
        local_state.colours.lone_integer.1,
        local_state.colours.lone_integer.2,
    ));
    help_text.push("- Assign value to variable\n".truecolor(
        local_state.colours.lone_fraction.0,
        local_state.colours.lone_fraction.1,
        local_state.colours.lone_fraction.2,
    ));
    help_text.push("  @name        ".truecolor(
        local_state.colours.lone_integer.0,
        local_state.colours.lone_integer.1,
        local_state.colours.lone_integer.2,
    ));
    help_text.push("- Use variable in expression\n".truecolor(
        local_state.colours.lone_fraction.0,
        local_state.colours.lone_fraction.1,
        local_state.colours.lone_fraction.2,
    ));

    // Examples
    help_text.push("\nExamples:\n".truecolor(
        local_state.colours.brackets.0,
        local_state.colours.brackets.1,
        local_state.colours.brackets.2,
    ));
    let examples = [
        ("2 + 2", "The meaning of life? Not quite, but it's a start."),
        (":base D", "Switch to base 13, because 12 bases are never enough."),
        ("6 * 9", "In Tridecimal, this might surprise you..."),
        ("#sin(@pi/4)", "For when your spaceship needs to make a 45, I mean 36-degree turn."),
        ("[3, 4] * [1, -1]", "Multiplying complex numbers: it's not rocket science, but it's close."),
        ("#sqrt-1", "The imaginary unit: i before @e, except after #sqrt."),
        ("1/2", "But why tho?"),
        (":base C", "Switch to base 12, see, tridecimal is weird."),
        ("1/2", "Ah, much better."),
        (":digits 10", "Adjust precision: for when you need to calculate the cost of a Pan Galactic Gargle Blaster to a dozen digits."),
        ("-6^(@pi/2) * #ln-2 + #sqrtB / #sin(2*@pi)", "Looks complex? That's because it is!"),
        (":base A", "Back to decimal. Phew!"),
        ("42", "The Answer. But what was the Question?"),
        ("&", "Use the previous result. Handy for building on your last calculation."),
        ("& + 1", "The Answer plus one. For those who always need a little extra."),
        ("@pi * 2", "Once around the universe."),
        ("#cos(2*@pi)", "Whoa, we've gone full circle!"),
        ("@e$@e", "Natural log of e - as natural as it gets!"),
        ("@rand", "Random number: perfect for simulating quantum improbability."),
        ("@grand", "Gaussian random: for when your probability needs to be normally distributed."),
        ("#floor(3.14159)", "Rounding down: because sometimes you need to be grounded."),
        ("@numfish=17%5", "Modulus: for when you need to know how many Babel fish are left."),
        ("#ceil(@numfish$2)", "How many bits needed for storing the number of fish? Let's find out!"),
        (":base G", "Hexadecimal: for the really hoopy froods."),
        ("FF", "The darkest shade in hex, or just 255 for the less cool."),
        ("FF$F", "And in nibbles, that's 2!"),
        (":base A", "And we're back to decimal. What a journey!"),
        ("&", "See?, 255.")
    ];

    for (example, desc) in examples.iter() {
        help_text.push(format!("- {}\n", desc).truecolor(
            local_state.colours.comma.0,
            local_state.colours.comma.1,
            local_state.colours.comma.2,
        ));
        help_text.push(format!("  {}\n", example).truecolor(
            local_state.colours.decimal.0,
            local_state.colours.decimal.1,
            local_state.colours.decimal.2,
        ));
        if example.starts_with(':') {
            // Handle commands
            match parse_command(example.as_bytes(), 1, &mut local_state) {
                CommandResult::Success(msg) => {
                    help_text.push(format!("  {}\n", msg).truecolor(
                        local_state.colours.message.0,
                        local_state.colours.message.1,
                        local_state.colours.message.2,
                    ));
                }
                CommandResult::Error(msg, _) => {
                    help_text.push(format!("  Error: {}\n", msg).truecolor(
                        local_state.colours.error.0,
                        local_state.colours.error.1,
                        local_state.colours.error.2,
                    ));
                }
                _ => {
                    // Nothing to show for silent commands (no example runs :help, :test or :dms)
                }
            }
        } else {
            // Handle expressions
            match tokenize(example, &mut local_state) {
                Ok(tokens) => {
                    match evaluate_tokens(&tokens, &mut local_state) {
                        Ok(result) => {
                            help_text.push("  ".normal());
                            let result_string = paint(&result_spans(&result, &local_state), &local_state.colours);
                            for part in result_string {
                                help_text.push(part);
                            }
                            help_text.push("\n".normal());
                            local_state.prev_result = result.value; // Update local_prev_result for & usage
                        }
                        Err(err) => {
                            help_text.push(format!("  Error: {}\n", err).truecolor(
                                local_state.colours.error.0,
                                local_state.colours.error.1,
                                local_state.colours.error.2,
                            ));
                        }
                    }
                }
                Err(halt) => {
                    help_text.push(format!("  Error: {}\n", halt.message()).truecolor(
                        local_state.colours.error.0,
                        local_state.colours.error.1,
                        local_state.colours.error.2,
                    ));
                }
            }
        }
        help_text.push("\n".normal());
    }

    help_text.push(
        "\nFor more information, comments, neat fractal renders, questions or or why 42, contact nick spiker."
            .normal(),
    );

    help_text
}
