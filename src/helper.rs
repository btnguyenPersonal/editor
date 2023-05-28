use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use crossterm::cursor::{SetCursorStyle, MoveTo, MoveToColumn, MoveToRow};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, SetAttribute, Attribute};
use crossterm::event::{KeyCode, KeyModifiers};
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader, Write};
use crossterm::terminal::size;
use std::process::{Command, Stdio};

pub fn get_clipboard_content() -> String {
    #[cfg(target_os = "macos")]
    let output = Command::new("pbpaste")
        .output()
        .expect("Failed to execute command");
    #[cfg(target_os = "linux")]
    let output = Command::new("xsel")
        .args(&["-o", "-b"]) // -o for output, -b for the clipboard
        .output()
        .expect("Failed to execute command");
    String::from_utf8(output.stdout).expect("Invalid UTF-8 data")
}

pub fn copy_to_clipboard(s: &str) -> Option<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("pbcopy");
    #[cfg(target_os = "linux")]
    let mut command = {
        // See https://linux.die.net/man/1/xclip.
        let mut c = Command::new("xclip");
        c.arg("-in");
        c.arg("-selection");
        c.arg("clipboard");
        c
    };
    let mut child = command.stdin(Stdio::piped()).spawn().ok()?;
    // When stdin is dropped the fd is automatically closed. See
    // https://doc.rust-lang.org/std/process/struct.ChildStdin.html.
    {
        let stdin = child.stdin.as_mut()?;
        stdin.write_all(s.as_bytes()).ok()?;
    }
    // Wait on pbcopy/xclip to finish.
    child.wait().ok()?;
    Some(())
}

pub fn get_file_data(file_name: &str) -> io::Result<Vec<String>> {
    let file_path = format!("{}", file_name);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    Ok(lines)
}

pub fn update_terminal(
    prev_render: &Vec<Vec<(char, Color, Color, bool)>>,
    current_render: &Vec<Vec<(char, Color, Color, bool)>>,
    full_render: bool) {

    let mut stdout = stdout();
    let (width, height) = size().expect("Failed to find terminal size");
    let height = height as usize;
    let width = width as usize;
    for y in 0..height {
        for x in 0..width {
            let (prev_char, prev_fg, prev_bg, prev_reverse) = if y < prev_render.len() && x < prev_render[y].len() {
                prev_render[y][x]
            } else {
                (' ', Color::White, Color::Black, false)
            };
            let (current_char, current_fg, current_bg, current_reverse) = if y < current_render.len() && x < current_render[y].len() {
                current_render[y][x]
            } else {
                (' ', Color::White, Color::Black, false)
            };
            if prev_char != current_char || prev_fg != current_fg || prev_bg != current_bg || prev_reverse != current_reverse || full_render {
                if current_reverse {
                    execute!(
                        stdout,
                        SetAttribute(Attribute::Reverse)
                    ).expect("Failed to set Reverse color");
                }
                execute!(
                    stdout,
                    MoveTo(x as u16, y as u16),
                    SetForegroundColor(current_fg),
                    SetBackgroundColor(current_bg),
                    Print(current_char),
                ).expect("Failed to update char");
                execute!(stdout, ResetColor).expect("Failed to set Reverse Color");
            }
        }
    }
}

pub fn render_file_data(
    prev_view: Vec<Vec<(char, Color, Color, bool)>>,
    file_name: &str,
    file_data: &[String],
    window_line_x: usize,
    window_line_y: usize,
    cursor_x: usize,
    cursor_y: usize,
    visual_x: usize,
    visual_y: usize,
    mode: char,
    search_string: String,
    searching: bool,
    macro_recording: bool,
    full_render: bool,
) -> Vec<Vec<(char, Color, Color, bool)>> {
    let mut stdout = stdout();
    if mode == 'i' {
        execute!(stdout, SetCursorStyle::SteadyBar).unwrap();
    } else {
        execute!(stdout, SetCursorStyle::DefaultUserShape).unwrap();
    }
    let terminal_size = size().unwrap();
    let term_height = terminal_size.1 as usize;
    let term_width = terminal_size.0 as usize;
    let mut screen_view: Vec<Vec<(char, Color, Color, bool)>> = Vec::new();
    let mut y = 0;
    let fg = if macro_recording {
        Color::Red
    } else {
        Color::DarkGrey
    };
    while y < term_height && window_line_y + y < file_data.len() {
        execute!(stdout, MoveToRow((y as u16).try_into().unwrap())).expect("Failed to move cursor");
        let mut line: String = if file_data[window_line_y + y].len() >= window_line_x {
            file_data[window_line_y + y][window_line_x..].to_string()
        } else {
            "".to_string()
        };
        if line.len() > term_width {
            let substring = &line[..term_width];
            line = String::from(substring);
        }
        let line_num_chars = format!("{:4} ", window_line_y + y + 1);
        let mut line_render = Vec::new();
        for num in line_num_chars.chars() {
            line_render.push((num, fg, Color::Black, false));
        }
        let line_chars = line.chars();
        let comment_string = match get_comment_string(file_name) {
            Some(str) => str,
            None => "#",
        };
        let comment_index = match find_substring(&line, comment_string) {
            Some(number) => number,
            None => usize::MAX,
        };
        let mut highlight = mode == 'V' && is_line_highlighted(y + window_line_y, visual_y, cursor_y);
        let mut fg_color = Color::White;
        let mut bg_color = Color::Black;
        let mut x = 0;
        let mut in_string = false;
        let mut string_char: char = '\0';
        let mut disregard_next = false;
        let search_len = search_string.len();
        let mut search_ranges: Vec<(usize, usize)> = vec![];
        for (index, _) in line.match_indices(&search_string) {
            search_ranges.push((index, index + search_len));
        }
        for (chr_index, chr) in line_chars.enumerate() {
            if mode == 'v' {
                highlight = is_highlighted(x + window_line_x, y + window_line_y, visual_x, visual_y, cursor_x, cursor_y);
            }
            if x >= comment_index {
                fg_color = Color::Green;
            } else {
                if in_string == true {
                    fg_color = Color::Magenta;
                }
                if chr == string_char && string_char != '\0' && !disregard_next {
                    in_string = !in_string;
                    string_char = '\0';
                } else if chr == '"' || chr == '\'' || chr == '`' {
                    fg_color = Color::Magenta;
                    if string_char == '\0' {
                        in_string = !in_string;
                        string_char = chr;
                    }
                } else if !in_string && (chr == '[' || chr == ']') {
                    fg_color = Color::Green;
                } else if !in_string && (chr == '{' || chr == '}') {
                    fg_color = Color::Cyan;
                } else if !in_string && (chr == '(' || chr == ')') {
                    fg_color = Color::Yellow;
                }
                disregard_next = false;
                if in_string && chr == '\\' {
                    disregard_next = true;
                }
                if searching && search_ranges.iter().any(|&(start, end)| chr_index >= start && chr_index < end) {
                    fg_color = Color::Black;
                    bg_color = Color::Green;
                } else {
                    bg_color = Color::Black;
                }
            }
            line_render.push((chr, fg_color, bg_color, highlight));
            fg_color = Color::White;
            x += 1;
        }
        if line.len() == 0 {
            line_render.push((' ', Color::White, Color::Black, highlight));
        }
        screen_view.push(line_render);
        y += 1;
    }
    update_terminal(&prev_view, &screen_view, full_render);
    execute!(stdout, MoveToRow(cursor_y as u16 - window_line_y as u16)).expect("Failed to move cursor");
    let cursor_x_display: u16 = if file_data[cursor_y].len() == 0 {
        0
    } else if cursor_x > file_data[cursor_y].len() {
        (file_data[cursor_y].len() - 1) as u16
    } else {
        cursor_x as u16 - window_line_x as u16
    };
    execute!(stdout, MoveToColumn(cursor_x_display as u16 + 5)).expect("Failed to move cursor");
    screen_view
}

pub fn starts_with_after_trim(my_string: &str, substring: &str) -> bool {
    let trimmed_string = my_string.trim_start();
    trimmed_string.starts_with(substring)
}

pub fn remove_substring_and_space_after(my_string: &str, substring: &str) -> String {
    if let Some(index) = my_string.find(substring) {
        let mut result = String::with_capacity(my_string.len());
        result.push_str(&my_string[..index]);
        if let Some(next_char) = my_string.get(index + substring.len()..).and_then(|s| s.chars().next()) {
            if next_char.is_whitespace() {
                result.push_str(&my_string[index + substring.len() + next_char.len_utf8()..]);
            } else {
                result.push_str(&my_string[index + substring.len()..]);
            }
        } else {
            result.push_str(&my_string[index + substring.len()..]);
        }
        result
    } else {
        String::from(my_string)
    }
}

pub fn toggle_comments_in_visual(file_data: &mut [String], comment_string: &str, cursor_y: usize, visual_y: usize) {
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    let mut lowest_indent = 0;
    let mut are_all_commented = true;
    for i in begin_y..end_y+1 {
        let current_indent = count_leading_spaces(&file_data[i].clone());
        if lowest_indent > current_indent {
            lowest_indent = current_indent;
        }
        if !starts_with_after_trim(&file_data[i].clone(), comment_string) && file_data[i].len() != 0 {
            are_all_commented = false;
        }
    }
    for i in begin_y..end_y+1 {
        if are_all_commented {
            file_data[i] = toggle_comment(file_data[i].clone(), comment_string);
        } else {
            file_data[i] = comment_at_index(file_data[i].clone(), comment_string, lowest_indent);
        }
    }
}

pub fn get_prev_occurrence(file_data: &[String], mut cursor_x: usize, mut cursor_y: usize, search_string: &str) -> Option<(usize, usize)> {
    let num_lines = file_data.len();
    let initial_y = cursor_y;
    let mut has_looped = false;
    loop {
        let line = &file_data[cursor_y];
        if cursor_x != 0 {
            if let Some(index) = line[..cursor_x-1].rfind(search_string) {
                return Some((index, cursor_y));
            }
        }
        if cursor_y == 0 {
            cursor_y = num_lines - 1;
            has_looped = true;
        } else {
            cursor_y -= 1;
        }
        cursor_x = file_data[cursor_y].len();
        if has_looped && cursor_y < initial_y {
            return None;
        }
    }
}

pub fn find_next_occurrence(file_data: &[String], mut cursor_x: usize, mut cursor_y: usize, search_string: &str) -> Option<(usize, usize)> {
    let num_lines = file_data.len();
    let initial_pos = cursor_y;
    let mut has_looped = false;
    loop {
        let line = &file_data[cursor_y];
        if cursor_x < line.len() {
            if let Some(index) = line[cursor_x..].find(search_string) {
                return Some((cursor_x + index, cursor_y));
            }
        }
        cursor_x = 0;
        cursor_y = (cursor_y + 1) % num_lines;
        if cursor_y == 0 {
            has_looped = true;
        }
        if has_looped && cursor_y > initial_pos {
            return None;
        }
    }
}

pub fn comment_at_index(mut line: String, comment_string: &str, index: usize) -> String {
    if line.len() != 0 {
        line.insert_str(index, " ");
        line.insert_str(index, comment_string);
        line
    } else {
        line
    }
}

pub fn toggle_comment(mut line: String, comment_string: &str) -> String {
    if line.len() != 0 {
        if !starts_with_after_trim(&line, comment_string) {
            let start = count_leading_spaces(&line);
            line.insert_str(start, " ");
            line.insert_str(start, comment_string);
            line
        } else {
            remove_substring_and_space_after(&line, comment_string)
        }
    } else {
        line
    }
}

pub fn get_comment_string(file_name: &str) -> Option<&str> {
    if let Some(extension) = file_name.split('.').last() {
        match extension {
            "scss" => Some("/*"),
            "css" => Some("/*"),
            "html" => Some("<!--"),
            "swift" => Some("//"),
            "java" => Some("//"),
            "js" => Some("//"),
            "jsx" => Some("//"),
            "ts" => Some("//"),
            "tsx" => Some("//"),
            "rs" => Some("//"),
            "c" => Some("//"),
            "cpp" => Some("//"),
            "sh" => Some("#"),
            "py" => Some("#"),
            "rb" => Some("#"),
            "pl" => Some("#"),
            "php" => Some("//"),
            "go" => Some("//"),
            "kotlin" => Some("//"),
            "groovy" => Some("//"),
            "scala" => Some("//"),
            "perl" => Some("#"),
            "lua" => Some("--"),
            "coffee" => Some("#"),
            "dart" => Some("//"),
            "h" => Some("/*"),
            "hpp" => Some("/*"),
            "m" => Some("//"),
            "mm" => Some("//"),
            "sql" => Some("--"),
            "vb" => Some("'"),
            "xml" => Some("<!--"),
            "yaml" => Some("#"),
            "dockerfile" => Some("#"),
            "makefile" => Some("#"),
            "bat" => Some("REM"),
            "powershell" => Some("#"),
            "ini" => Some(";"),
            "cmake" => Some("#"),
            "m4" => Some("dnl"),
            "fish" => Some("#"),
            "haskell" => Some("--"),
            "elixir" => Some("#"),
            "nim" => Some("#"),
            "erl" => Some("%"),
            "ex" => Some("#"),
            "f" => Some("!"),
            "fort" => Some("C"),
            "f90" => Some("!"),
            "f95" => Some("!"),
            "hs" => Some("--"),
            "ml" => Some("(*"),
            "mli" => Some("(*"),
            "scm" => Some(";;"),
            "ss" => Some(";;"),
            "tcl" => Some("#"),
            "vim" => Some("\""),
            _ => None,
        }
    } else {
        None
    }
}

pub fn find_substring(line: &str, substring: &str) -> Option<usize> {
    if let Some(index) = line.find(substring) {
        Some(index)
    } else {
        None
    }
}

pub fn log_command(code: KeyCode, modifiers: KeyModifiers, last_command: &mut Vec<(KeyCode, KeyModifiers)>, recording: bool) {
    if recording {
        last_command.push((code, modifiers));
    }
}

pub fn quit_terminal() {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen).expect("Failed to leave alternate screen");
    disable_raw_mode().expect("Failed to disable raw mode");
}

pub fn save_to_file(data: &[String], file_path: &str) {
    if let Ok(mut file) = File::create(file_path) {
        for line in data {
            let _ = file.write_all(line.as_bytes());
            let _ = file.write_all(b"\n");
        }
    } else {
        println!("Failed to save the file");
    }
}

pub fn get_in_word(string: &str, cursor_x: usize) -> Option<(usize, usize)> {
    let string_chars: Vec<char> = string.chars().collect();
    let mut word_start: Option<usize> = None;
    let mut word_end: Option<usize> = None;
    let mut index = cursor_x;
    loop {
        if string_chars[index].is_alphanumeric() || string_chars[index] == '_' {
            word_start = Some(index);
            if index == 0 {
                break;
            }
            index -= 1;
        } else {
            break;
        }
    }
    for (index, &character) in string_chars.iter().enumerate().skip(cursor_x) {
        let is_in_word = character.is_alphanumeric() || character == '_';
        if is_in_word && word_start.is_none() {
            word_start = Some(index);
        } else if !is_in_word && word_start.is_some() {
            word_end = if index == 0 { Some(0) } else { Some(index - 1) };
            break;
        }
    }
    if word_start.is_some() && word_end.is_none() {
        word_end = if string_chars.len() == 0 { Some(0) } else { Some(string_chars.len() - 1) };
    }
    word_start.zip(word_end)
}

pub fn right_insert(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if cursor_x + 1 <= file_data[cursor_y].len() {
        cursor_x + 1
    } else {
        cursor_x
    }
}

pub fn right(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if cursor_x + 1 < file_data[cursor_y].len() {
        cursor_x + 1
    } else {
        cursor_x
    }
}

pub fn left(cursor_x: usize) -> usize {
    if cursor_x > 0 {
        cursor_x - 1
    } else {
        cursor_x
    }
}

pub fn down(file_data: &[String], cursor_y: usize) -> usize {
    if cursor_y < file_data.len() - 1 {
        cursor_y + 1
    } else {
        cursor_y
    }
}

pub fn up(cursor_y: usize) -> usize {
    if cursor_y > 0 {
        cursor_y - 1
    } else {
        cursor_y
    }
}

pub fn get_index_prev_word(file_data: &[String], mut cursor_x: usize, cursor_y: usize) -> usize {
    let cursor_start = cursor_x;
    let line = &file_data[cursor_y];
    if cursor_x == 0 {
        return cursor_start;
    }
    let current = line.chars().nth(cursor_x);
    if current == None {
        return cursor_start;
    }
    cursor_x = get_index_prev_alpha_word(&file_data, cursor_x, cursor_y);
    let prev_word_start = get_index_prev_non_word(&file_data, cursor_x, cursor_y);
    if prev_word_start == cursor_x {
        return 0;
    }
    prev_word_start + 1
}

pub fn get_index_prev_alpha_word(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    let line = &file_data[cursor_y];
    let line_slice = &line[..cursor_x];
    if let Some(prev_word_start) = line_slice.char_indices().rev().find(|(_, c)| c.is_alphanumeric() || *c == '_').map(|(i, _)| i) {
        prev_word_start
    } else {
        cursor_x
    }
}

pub fn get_index_prev_non_word(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    let line = &file_data[cursor_y];
    let line_slice = &line[..cursor_x];
    if let Some(prev_word_start) = line_slice.char_indices().rev().find(|(_, c)| !c.is_alphanumeric() && *c != '_').map(|(i, _)| i) {
        prev_word_start
    } else {
        cursor_x
    }
}

pub fn get_index_next_word(file_data: &[String], mut cursor_x: usize, cursor_y: usize) -> usize {
    let cursor_start = cursor_x;
    let line = &file_data[cursor_y];
    let current = line.chars().nth(cursor_x);
    if current == None {
        return cursor_start;
    }
    if current.unwrap().is_alphanumeric() || current.unwrap() == '_' {
        cursor_x = get_index_next_non_word(&file_data, cursor_x, cursor_y);
    }
    let line_slice = &line[cursor_x..];
    if let Some(next_word_start) = line_slice.find(|c: char| c.is_alphanumeric() || c == '_') {
        cursor_x + next_word_start
    } else {
        cursor_start
    }
}

pub fn get_next_empty_line(file_data: &[String], cursor_y: usize) -> usize {
    for i in cursor_y+1..file_data.len() {
        if file_data[i] == "" {
            return i;
        }
    }
    return file_data.len() - 1;
}

pub fn get_prev_empty_line(file_data: &[String], cursor_y: usize) -> usize {
    for i in (0..cursor_y).rev() {
        if file_data[i] == "" {
            return i;
        }
    }
    return 0;
}

pub fn get_index_next_non_word(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    let line = &file_data[cursor_y];
    let line_slice = &line[cursor_x..];
    if let Some(next_word_start) = line_slice.find(|c: char| !c.is_alphanumeric() && c != '_') {
        cursor_x + next_word_start
    } else {
        cursor_x
    }
}

pub fn set_cursor_end(file_data: &[String], cursor_y: usize) -> usize {
    file_data[cursor_y].len()
}

pub fn prevent_cursor_end(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if file_data[cursor_y].len() != 0 && cursor_x >= file_data[cursor_y].len() {
        file_data[cursor_y].len() - 1
    } else {
        cursor_x
    }
}

pub fn reset_cursor_end(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if file_data[cursor_y].len() == 0 {
        0
    } else if cursor_x > file_data[cursor_y].len() - 1 {
        file_data[cursor_y].len() - 1
    } else {
        cursor_x
    }
}

pub fn increase_indent(string: String) -> String {
    if string == "" {
        "    ".to_string()
    } else {
        format!("    {}", string)
    }
}

pub fn reduce_indent(string: String) -> String {
    if string.chars().take(4).all(|c| c == ' ') {
        string.chars().skip(4).collect()
    } else {
        string.trim_start().to_string()
    }
}

pub fn normalize(cursor: usize, visual: usize) -> (usize, usize) {
    if cursor <= visual {
        (cursor, visual)
    } else {
        (visual, cursor)
    }
}

pub fn increase_indent_visual(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize) {
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for i in begin_y..end_y+1 {
        file_data[i] = increase_indent(file_data[i].clone());
    }
}

pub fn reduce_indent_visual(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize) {
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for i in begin_y..end_y+1 {
        file_data[i] = reduce_indent(file_data[i].clone());
    }
}

pub fn calc_window_lines(file_data: &[String], window_line_x: usize, window_line_y: usize, cursor_x: usize, cursor_y: usize) -> (usize, usize) {
    let terminal_size = size().unwrap();
    let term_height = terminal_size.1 as usize;
    let term_width = terminal_size.0 as usize;
    let mut x = window_line_x;
    let mut y = window_line_y;
    let cursor_display_x = if cursor_x > file_data[cursor_y].len() { file_data[cursor_y].len() } else { cursor_x };
    if window_line_x + (term_width - 6) <= cursor_display_x {
        x = cursor_display_x - (term_width - 6);
    }
    if window_line_x > cursor_display_x {
        x = cursor_display_x;
    }
    if window_line_y > cursor_y {
        y = cursor_y;
    }
    if window_line_y + term_height <= cursor_y {
        y = cursor_y - term_height + 1;
    }
    (x, y)
}

pub fn is_highlighted(x: usize, y: usize, visual_x: usize, visual_y: usize, cursor_x: usize, cursor_y: usize) -> bool {
    y < visual_y && y > cursor_y || y > visual_y && y < cursor_y
    || (y == cursor_y && y == visual_y && visual_y >= cursor_y && x >= visual_x && x <= cursor_x)
    || (y == cursor_y && y == visual_y && visual_y <= cursor_y && x <= visual_x && x >= cursor_x)
    || (y == cursor_y && cursor_y < visual_y && x >= cursor_x)
    || (y == cursor_y && cursor_y > visual_y && x <= cursor_x)
    || (y == visual_y && cursor_y < visual_y && x <= visual_x)
    || (y == visual_y && cursor_y > visual_y && x >= visual_x)
}

pub fn is_line_highlighted(y: usize, visual_y: usize, cursor_y: usize) -> bool {
    y <= visual_y && y >= cursor_y || y >= visual_y && y <= cursor_y
}

pub fn get_cursor_after_visual(cursor: usize, visual: usize) -> usize {
    if cursor <= visual {
        cursor
    } else {
        visual
    }
}

pub fn paste_before(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize) {
    let mut clip = get_clipboard_content();
    if clip.starts_with("\n") {
        clip.remove(0);
        let lines: Vec<&str> = clip.split('\n').collect();
        for line in lines.iter().rev() {
            let _ = &file_data.insert(cursor_y, line.to_string());
        }
    } else {
        if cursor_x <= file_data[cursor_y].len() {
            let lines: Vec<&str> = clip.split('\n').collect();
            let mut end = lines.last().expect("Can't get last").to_string();
            end += &file_data[cursor_y][cursor_x..].to_string();
            file_data[cursor_y] = file_data[cursor_y][..cursor_x].to_string();
            file_data[cursor_y] += lines[0];
            let mut y = 1;
            let _ = &file_data.insert(cursor_y + 1, end.to_string());
            while y < lines.len() - 1 {
                let _ = &file_data.insert(cursor_y + 1, lines[y].to_string());
                y += 1;
            }
        }
    }
}

pub fn paste_after(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize) {
    let mut clip = get_clipboard_content();
    if clip.starts_with("\n") {
        clip.remove(0);
        let lines: Vec<&str> = clip.split('\n').collect();
        for line in lines.iter().rev() {
            let _ = &file_data.insert(cursor_y + 1, line.to_string());
        }
    } else {
        if cursor_x <= file_data[cursor_y].len() {
            let lines: Vec<&str> = clip.split('\n').collect();
            let mut end = lines.last().expect("Can't get last").to_string();
            end += &file_data[cursor_y][cursor_x+1..].to_string();
            file_data[cursor_y] = file_data[cursor_y][..cursor_x+1].to_string();
            file_data[cursor_y] += lines[0];
            let mut y = 1;
            let _ = &file_data.insert(cursor_y + 1, end.to_string());
            while y < lines.len() - 1 {
                let _ = &file_data.insert(cursor_y + 1, lines[y].to_string());
                y += 1;
            }
        }
    }
}

pub fn copy_in_visual(
    file_data: &mut Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    visual_x: usize,
    visual_y: usize,
    mode: char
    ) -> (usize, usize) {
    let mut clipboard: String = if mode == 'V' {"\n".to_string()} else {"".to_string()};
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    let (begin_x, end_x) = if cursor_x <= visual_x {
        (cursor_x, visual_x)
    } else {
        (visual_x, cursor_x)
    };
    if mode == 'V' {
        for i in begin_y..end_y {
            clipboard += &file_data[i];
            clipboard += "\n";
        }
        clipboard += &file_data[end_y];
    } else if mode == 'v' {
        if begin_y == end_y {
            clipboard += &file_data[begin_y][begin_x..end_x+1];
        } else {
            clipboard += &file_data[begin_y][begin_x..];
            for i in begin_y..end_y-1 {
                clipboard += "\n";
                clipboard += &file_data[i];
            }
            clipboard += "\n";
            if file_data[end_y].len() > 0 {
                clipboard += &file_data[end_y][..end_x+1];
            }
        }
    }
    copy_to_clipboard(&clipboard).expect("failed copy to clipboard");
    (begin_x, begin_y)
}

pub fn delete_in_visual(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize, visual_x: usize, visual_y: usize, mode: char) -> (usize, usize) {
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    let (begin_x, end_x) = if cursor_x <= visual_x {
        (cursor_x, visual_x)
    } else {
        (visual_x, cursor_x)
    };
    if mode == 'V' {
        for _ in begin_y..=end_y {
            file_data.remove(begin_y);
        }
    } else if mode == 'v' {
        if begin_y == end_y {
            file_data[begin_y].drain(begin_x..end_x+1);
        } else {
            file_data[begin_y].drain(begin_x..);
            for _ in begin_y..end_y-1 {
                file_data.remove(begin_y + 1);
            }
            if file_data[begin_y+1].len() > 0 {
                file_data[begin_y+1].drain(..end_x+1);
                let joined_lines = file_data[begin_y..begin_y+2].join("");
                file_data[begin_y] = joined_lines.to_string();
            }
            file_data.remove(begin_y + 1);
        }
    }
    if file_data.len() == 0 {
        file_data.insert(0, "".to_string());
    }
    (begin_x, begin_y)
}

pub fn delete_in_visual_and_insert(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize) {
    let (begin, end) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for _ in begin..=end {
        file_data.remove(begin);
    }
    file_data.insert(begin, "".to_string());
}

pub fn reset_cursor_end_file(length: usize, cursor_y: usize) -> usize {
    if cursor_y >= length {
        length - 1
    } else {
        cursor_y
    }
}

pub fn count_leading_spaces(input: &str) -> usize {
    let trimmed = input.trim_start();
    let leading_spaces = input.len() - trimmed.len();
    leading_spaces
}

