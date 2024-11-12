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
// - Track precision loss through operation chains
// - Save/load common transformation sequences
// - Built-in coordinate system transforms, int/float conversions, etc.
// - All common operators should be defined and integrated
//
// 4. Syntax Suggestions:
// - Store value: -> var_name
// - Assert within epsilon: =~ expected_value
// - Show precision loss: .precision
// - Save sequence: .save transform_name

use az::Cast;
use colored::*;
use dirs;
use rug::ops::*;
use rug::*;
use std::fs;
use std::io::{self, Write};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
                                let result_vec = if let Some(var_idx) = result.assignment {
                                    // For assignments, prepend the variable name
                                    let mut vec = vec![format!("@{} = ", state.variables[var_idx].name)
                                        .truecolor(state.colours.message.0, state.colours.message.1, state.colours.message.2)];
                                    vec.extend(num2string(&result.value, &state));
                                    vec
                                } else {
                                    num2string(&result.value, &state)
                                };
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
                    Err((msg, pos)) => {
                        if pos == std::usize::MAX {
                            println!(
                                "{}",
                                msg.truecolor(
                                    state.colours.message.0,
                                    state.colours.message.1,
                                    state.colours.message.2
                                )
                            );
                        } else {
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
                    }
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
struct EvalResult {
    value: Complex,
    assignment: Option<usize>, // Index of assigned variable, if this was an assignment
}
#[derive(Clone)]
struct Variable {
    name: String,
    value: Complex,
}
#[derive(Clone)]
struct BasecalcState {
    base: u8,
    digits: usize,
    precision: u32,
    padding: u32,
    radians: bool,
    current_entry: String,
    history_index: usize,
    history: Vec<String>,
    debug: bool,
    rand_state: rand::RandState<'static>,
    prev_result: Complex,
    colours: RGBValues,
    variables: Vec<Variable>,
}

impl BasecalcState {
    fn new() -> Self {
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
    fn set_precision(&mut self) {
        self.precision =
            (self.digits as f64 * (self.base as f64).log2()).ceil() as u32 + self.padding;
    }
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
static OPERATORS: [(&str, char, u8, &str); 30] = [
    // Basic arithmetic
    ("+", '+', 2, "addition"),
    ("-", '-', 2, "subtraction"),
    ("*", '*', 2, "multiplication"),
    ("/", '/', 2, "division"),
    ("^", '^', 2, "exponentiation"),
    ("%", '%', 2, "modulus"),
    ("$", '$', 2, "log and base logarithm"),
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
static CONSTANTS: [(&str, char, &str); 7] = [
    ("@pi", 'p', "Pi"),
    ("@phi", 'P', "Golden ratio"),
    ("@e", 'E', "Euler's number"),
    ("@gamma", 'G', "Euler-Mascheroni constant"),
    ("@rand", 'r', "Random number between 0 and 1"),
    ("@grand", 'g', "Gaussian random number"),
    ("&", '&', "Previous result"),
];
#[derive(Clone)]
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
    Assignment,
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
    var_index: Option<usize>,
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
            var_index: None,
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
fn tokenize(input_str: &str, state: &mut BasecalcState) -> Result<Vec<Token>, (String, usize)> {
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
                CommandResult::Success(msg) => return Err((msg, std::usize::MAX)),
                CommandResult::Error(msg, pos) => return Err((msg, pos)),
                CommandResult::Silent => return Err(("".to_string(), std::usize::MAX)),
            }
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
fn evaluate_tokens(tokens: &[Token], state: &mut BasecalcState) -> Result<EvalResult, String> {
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
fn apply_operator(
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
fn get_precedence(op: char) -> Precedence {
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
fn apply_unary_operator(
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
fn apply_binary_operator(output_queue: &mut Vec<Complex>, op: char) -> Result<(), String> {
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
fn parse_constant(
    input: &[u8],
    index: usize,
    state: &mut BasecalcState,
) -> Result<(Token, usize), (String, usize)> {
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
    if input[index] == b'@' {
        let mut var_name = String::new();
        let mut curr_index = index + 1;
        
        // Parse variable name
        while curr_index < input.len() {
            let c = input[curr_index];
            if !c.is_ascii_alphanumeric() && c != b'_' {
                break;
            }
            var_name.push(c as char);
            curr_index += 1;
        }

        if var_name.is_empty() {
            return Err(("Invalid variable name!".to_string(), index));
        }

        // Look for existing variable
        if let Some(pos) = state.variables.iter().position(|v| v.name == var_name) {
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
        while look_ahead < input.len() && input[look_ahead].is_ascii_whitespace() {
            look_ahead += 1;
        }

        if look_ahead < input.len() && input[look_ahead] == b'=' {
            // This is an assignment - create new variable
            state.variables.push(Variable {
                name: var_name,
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
enum CommandResult {
    /// Command was successful, with a message to display
    Success(String),
    /// Command failed, with an error message and the position of the error
    Error(String, usize),
    /// Command was successful but requires no message (like :help)
    Silent,
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
fn parse_command(input: &[u8], mut index: usize, state: &mut BasecalcState) -> CommandResult {
    match &input[index..] {
        s if s.eq_ignore_ascii_case(b"test") => {
            let (passed, total) = run_tests();
            CommandResult::Success(format!("{}/{} tests passed.", passed, total))
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
            let dms = num2dms(&state.prev_result, state);
            for block in dms {
                print!("{}", block);
            }
            CommandResult::Success("".to_string())
        }
        s if s.eq_ignore_ascii_case(b"help") => {
            let help_text = get_help_text(&state);
            for line in help_text {
                print!("{}", line);
            }
            println!("\n");
            print_settings(state);
            CommandResult::Silent
        }
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
        let value_string = num2string(&value, &local_state);

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
                CommandResult::Silent => {
                    // Do nothing for silent commands
                }
            }
        } else {
            // Handle expressions
            match tokenize(example, &mut local_state) {
                Ok(tokens) => {
                    match evaluate_tokens(&tokens, &mut local_state) {
                        Ok(result) => {
                            help_text.push("  ".normal());
                            let result_string = if let Some(var_idx) = result.assignment {
                                let mut vec = vec![format!("@{} = ", local_state.variables[var_idx].name)
                                    .truecolor(
                                        local_state.colours.message.0,
                                        local_state.colours.message.1,
                                        local_state.colours.message.2,
                                    )];
                                vec.extend(num2string(&result.value, &local_state));
                                vec
                            } else {
                                num2string(&result.value, &local_state)
                            };
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
                Err((msg, _)) => {
                    help_text.push(format!("  Error: {}\n", msg).truecolor(
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
/// * `state` - The current calculator state
///
/// # Returns
/// * `Complex` - The complex number representation of the token
fn token2num(token: &Token, state: &mut BasecalcState) -> Complex {
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
fn num2string(num: &Complex, state: &BasecalcState) -> Vec<ColoredString> {
    let mut result = Vec::new();

    if num.real().is_nan()
        || num.imag().is_nan()
        || num.real().is_infinite()
        || num.imag().is_infinite()
    {
        result.push("NaN".truecolor(
            state.colours.nan.0,
            state.colours.nan.1,
            state.colours.nan.2,
        ));
        return result;
    }

    if num.imag().is_zero() {
        result.push(" ".normal());
        result.extend(format_part(num.real(), state, true, true));
    } else {
        result.push("[".truecolor(
            state.colours.brackets.0,
            state.colours.brackets.1,
            state.colours.brackets.2,
        ));
        result.extend(format_part(num.real(), state, true, false));
        result.push(" ,".truecolor(
            state.colours.comma.0,
            state.colours.comma.1,
            state.colours.comma.2,
        ));
        result.extend(format_part(num.imag(), state, false, false));
        result.push(" ]".truecolor(
            state.colours.brackets.0,
            state.colours.brackets.1,
            state.colours.brackets.2,
        ));
    }

    result
}
/// Converts a complex number to a vector of DMS coloured strings for display
///
/// # Arguments
/// * `num` - The complex number to convert
/// * `base` - The current number base
/// * `digits` - The number of digits to display
/// * `colours` - The colour scheme for output formatting
///
/// # Returns
/// * `Vec<ColoredString>` - A vector of coloured strings representing the number
fn num2dms(num: &Complex, state: &BasecalcState) -> Vec<ColoredString> {
    let mut result = Vec::new();

    if num.real().is_nan()
        || num.imag().is_nan()
        || num.real().is_infinite()
        || num.imag().is_infinite()
    {
        result.push("NaN".truecolor(
            state.colours.nan.0,
            state.colours.nan.1,
            state.colours.nan.2,
        ));
        return result;
    }

    if num.imag().is_zero() {
        result.push(" ".normal());
        result.extend(format_dms(num.real(), state, true, true));
    } else {
        result.push("[".truecolor(
            state.colours.brackets.0,
            state.colours.brackets.1,
            state.colours.brackets.2,
        ));
        result.extend(format_dms(num.real(), state, true, false));
        result.push(" ,".truecolor(
            state.colours.comma.0,
            state.colours.comma.1,
            state.colours.comma.2,
        ));
        result.extend(format_dms(num.imag(), state, false, false));
        result.push(" ]".truecolor(
            state.colours.brackets.0,
            state.colours.brackets.1,
            state.colours.brackets.2,
        ));
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
    state: &BasecalcState,
    is_real: bool,
    is_lone: bool,
) -> Vec<ColoredString> {
    let mut result = Vec::new();

    if num.is_zero() {
        result.push(" ".normal());
        result.push("0".truecolor(
            state.colours.lone_integer.0,
            state.colours.lone_integer.1,
            state.colours.lone_integer.2,
        ));
        result.push(".".truecolor(
            state.colours.decimal.0,
            state.colours.decimal.1,
            state.colours.decimal.2,
        ));
        return result;
    }
    if num.is_nan() || num.is_infinite() {
        result.push("NaN".truecolor(
            state.colours.nan.0,
            state.colours.nan.1,
            state.colours.nan.2,
        ));
        return result;
    }

    let is_positive = num.is_sign_positive();
    if is_positive {
        result.push(" ".normal());
    } else {
        result.push("-".truecolor(
            state.colours.sign.0,
            state.colours.sign.1,
            state.colours.sign.2,
        ));
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
    let (int_colour, frac_colour) = if is_lone {
        (state.colours.lone_integer, state.colours.lone_fraction)
    } else if is_real {
        (state.colours.real_integer, state.colours.real_fraction)
    } else {
        (
            state.colours.imaginary_integer,
            state.colours.imaginary_fraction,
        )
    };
    let prec = num_abs.prec();
    let tilde = (num_abs * Float::with_val(prec, 2) - Float::with_val(prec, state.base)).abs()
        > 2f64.pow(-16);
    if decimal {
        if integer_part.is_empty() {
            result.push("0".truecolor(int_colour.0, int_colour.1, int_colour.2));
        } else {
            result.push(integer_part.truecolor(int_colour.0, int_colour.1, int_colour.2));
        }
        result.push(".".truecolor(
            state.colours.decimal.0,
            state.colours.decimal.1,
            state.colours.decimal.2,
        ));
        result.push(trim_zeros(fractional_part).truecolor(
            frac_colour.0,
            frac_colour.1,
            frac_colour.2,
        ));
        if tilde {
            result.push("~".truecolor(
                state.colours.tilde.0,
                state.colours.tilde.1,
                state.colours.tilde.2,
            ));
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
                result.push("~".truecolor(
                    state.colours.tilde.0,
                    state.colours.tilde.1,
                    state.colours.tilde.2,
                ));
            } else {
                result.push(" ".normal());
            }
            result.push(" :".truecolor(
                state.colours.colon.0,
                state.colours.colon.1,
                state.colours.colon.2,
            ));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, state.base as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
                ));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, state.base as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
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
                result.push("~".truecolor(
                    state.colours.tilde.0,
                    state.colours.tilde.1,
                    state.colours.tilde.2,
                ));
            } else {
                result.push(" ".normal());
            }
            result.push(" :".truecolor(
                state.colours.colon.0,
                state.colours.colon.1,
                state.colours.colon.2,
            ));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, state.base as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
                ));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, state.base as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
                ));
            }
        }
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
/// * `Vec<ColoredString>` - A vector of coloured strings representing the formatted DMS part
fn format_dms(
    num: &rug::Float,
    state: &BasecalcState,
    is_real: bool,
    is_lone: bool,
) -> Vec<ColoredString> {
    let mut result = Vec::new();

    if num.is_zero() {
        result.push(" ".normal());
        result.push("Zil".truecolor(
            state.colours.lone_integer.0,
            state.colours.lone_integer.1,
            state.colours.lone_integer.2,
        ));
        result.push(".".truecolor(
            state.colours.decimal.0,
            state.colours.decimal.1,
            state.colours.decimal.2,
        ));
        return result;
    }
    if num.is_nan() || num.is_infinite() {
        result.push("NaN".truecolor(
            state.colours.nan.0,
            state.colours.nan.1,
            state.colours.nan.2,
        ));
        return result;
    }

    let is_positive = num.is_sign_positive();
    if is_positive {
        result.push(" ".normal());
    } else {
        result.push("-".truecolor(
            state.colours.sign.0,
            state.colours.sign.1,
            state.colours.sign.2,
        ));
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
    let (int_colour, frac_colour) = if is_lone {
        (state.colours.lone_integer, state.colours.lone_fraction)
    } else if is_real {
        (state.colours.real_integer, state.colours.real_fraction)
    } else {
        (
            state.colours.imaginary_integer,
            state.colours.imaginary_fraction,
        )
    };
    let prec = num_abs.prec();
    let tilde =
        (num_abs * Float::with_val(prec, 2) - Float::with_val(prec, 12)).abs() > 2f64.pow(-16);
    if decimal {
        if integer_part.is_empty() {
            result.push("Zil".truecolor(int_colour.0, int_colour.1, int_colour.2));
        } else {
            result.push(integer_part.truecolor(int_colour.0, int_colour.1, int_colour.2));
        }
        result.push(".".truecolor(
            state.colours.decimal.0,
            state.colours.decimal.1,
            state.colours.decimal.2,
        ));
        result.push(trim_zeros(fractional_part).truecolor(
            frac_colour.0,
            frac_colour.1,
            frac_colour.2,
        ));
        if tilde {
            result.push("~".truecolor(
                state.colours.tilde.0,
                state.colours.tilde.1,
                state.colours.tilde.2,
            ));
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
                result.push("~".truecolor(
                    state.colours.tilde.0,
                    state.colours.tilde.1,
                    state.colours.tilde.2,
                ));
            } else {
                result.push(" ".normal());
            }
            result.push(" :".truecolor(
                state.colours.colon.0,
                state.colours.colon.1,
                state.colours.colon.2,
            ));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, 12 as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
                ));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, 12 as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
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
                result.push("~".truecolor(
                    state.colours.tilde.0,
                    state.colours.tilde.1,
                    state.colours.tilde.2,
                ));
            } else {
                result.push(" ".normal());
            }
            result.push(" :".truecolor(
                state.colours.colon.0,
                state.colours.colon.1,
                state.colours.colon.2,
            ));
            if decimal_place < 0 {
                let mut exponent = "-".to_owned();
                exponent.push_str(&format_int((-decimal_place) as usize, 12 as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
                ));
            } else {
                let mut exponent = " ".to_owned();
                exponent.push_str(&format_int(decimal_place as usize, 12 as usize));
                result.push(exponent.truecolor(
                    state.colours.exponent.0,
                    state.colours.exponent.1,
                    state.colours.exponent.2,
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
fn run_tests() -> (usize, usize) {
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
    let mut passed = 0;
    let total = tests.len();
    for (input, expected) in tests {
        println!("> {}", input);

        let (coloured_result, result) = match tokenize(input, &mut state) {
            Ok(tokens) => match evaluate_tokens(&tokens, &mut state) {
                Ok(result) => {
                    let coloured_vec = if let Some(var_idx) = result.assignment {
                        let mut vec = vec![format!("@{} = ", state.variables[var_idx].name)
                            .truecolor(state.colours.message.0, state.colours.message.1, state.colours.message.2)];
                        vec.extend(num2string(&result.value, &state));
                        vec
                    } else {
                        num2string(&result.value, &state)
                    };
                    state.prev_result = result.value;
                    (coloured_vec.clone(), coloured_vec_to_string(&coloured_vec))
                }
                Err(err) => (vec![err.red()], err),
            },
            Err((msg, _)) => (
                vec![msg.truecolor(
                    state.colours.message.0,
                    state.colours.message.1,
                    state.colours.message.2,
                )],
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
