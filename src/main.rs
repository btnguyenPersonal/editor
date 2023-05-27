use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::cursor::{MoveTo, MoveToColumn, MoveToRow};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, SetAttribute, Attribute};
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader, Write};
use crossterm::terminal::size;
use std::process::{Command, Stdio};

fn get_clipboard_content() -> String {
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

fn copy_to_clipboard(s: &str) -> Option<()> {
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

fn get_file_data(file_name: &str) -> io::Result<Vec<String>> {
    let file_path = format!("{}", file_name);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    Ok(lines)
}

fn update_terminal(
    prev_render: &Vec<Vec<(char, Color, Color, bool)>>,
    current_render: &Vec<Vec<(char, Color, Color, bool)>>) {

    let mut stdout = stdout();
    // execute!(stdout, Hide).expect("Failed to hide cursor");
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
            if prev_char != current_char || prev_fg != current_fg || prev_bg != current_bg || prev_reverse != current_reverse {
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
    // execute!(stdout, Show).expect("Failed to show cursor");
}

fn render_file_data(
    prev_view: Vec<Vec<(char, Color, Color, bool)>>,
    file_data: &[String],
    window_line_x: usize,
    window_line_y: usize,
    cursor_x: usize,
    cursor_y: usize,
    visual_x: usize,
    visual_y: usize,
    mode: char
) -> Vec<Vec<(char, Color, Color, bool)>> {
    let mut stdout = stdout();
    let terminal_size = size().unwrap();
    let term_height = terminal_size.1 as usize;
    let term_width = terminal_size.0 as usize;
    let mut screen_view: Vec<Vec<(char, Color, Color, bool)>> = Vec::new();
    let mut y = 0;
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
            line_render.push((num, Color::DarkGrey, Color::Black, false));
        }
        let line_chars = line.chars();
        let mut highlight = mode == 'V' && is_line_highlighted(y + window_line_y, visual_y, cursor_y);
        let mut fg_color = Color::White;
        let mut x = 0;
        let mut in_string = false;
        let mut string_char: char = '\0';
        for chr in line_chars {
            if mode == 'v' {
                highlight = is_highlighted(x + window_line_x, y + window_line_y, visual_x, visual_y, cursor_x, cursor_y);
            }
            if in_string == true {
                fg_color = Color::Magenta;
            }
            // TODO comments and escape chars
            if chr == string_char && string_char != '\0' {
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
            line_render.push((chr, fg_color, Color::Black, highlight));
            fg_color = Color::White;
            x += 1;
        }
        if line.len() == 0 {
            line_render.push((' ', Color::White, Color::Black, highlight));
        }
        screen_view.push(line_render);
        y += 1;
    }
    update_terminal(&prev_view, &screen_view);
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

fn quit_terminal() {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen).expect("Failed to leave alternate screen");
    disable_raw_mode().expect("Failed to disable raw mode");
}

fn save_to_file(data: &[String], file_path: &str) {
    if let Ok(mut file) = File::create(file_path) {
        for line in data {
            let _ = file.write_all(line.as_bytes());
            let _ = file.write_all(b"\n");
        }
    } else {
        println!("Failed to save the file");
    }
}

fn right(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if cursor_x + 1 < file_data[cursor_y].len() {
        cursor_x + 1
    } else {
        cursor_x
    }
}

fn left(cursor_x: usize) -> usize {
    if cursor_x > 0 {
        cursor_x - 1
    } else {
        cursor_x
    }
}

fn down(file_data: &[String], cursor_y: usize) -> usize {
    if cursor_y < file_data.len() - 1 {
        cursor_y + 1
    } else {
        cursor_y
    }
}

fn up(cursor_y: usize) -> usize {
    if cursor_y > 0 {
        cursor_y - 1
    } else {
        cursor_y
    }
}

fn set_cursor_end(file_data: &[String], cursor_y: usize) -> usize {
    file_data[cursor_y].len()
}

fn prevent_cursor_end(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if file_data[cursor_y].len() != 0 && cursor_x >= file_data[cursor_y].len() {
        file_data[cursor_y].len() - 1
    } else {
        cursor_x
    }
}

fn reset_cursor_end(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if file_data[cursor_y].len() == 0 {
        0
    } else if cursor_x > file_data[cursor_y].len() - 1 {
        file_data[cursor_y].len() - 1
    } else {
        cursor_x
    }
}

fn increase_indent(string: String) -> String {
    format!("    {}", string)
}

fn reduce_indent(string: String) -> String {
    if string.chars().take(4).all(|c| c == ' ') {
        string.chars().skip(4).collect()
    } else {
        string.trim_start().to_string()
    }
}

fn increase_indent_visual(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize) {
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for i in begin_y..end_y+1 {
        file_data[i] = increase_indent(file_data[i].clone());
    }
}

fn reduce_indent_visual(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize) {
    let (begin_y, end_y) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for i in begin_y..end_y+1 {
        file_data[i] = reduce_indent(file_data[i].clone());
    }
}

fn calc_window_lines(file_data: &[String], window_line_x: usize, window_line_y: usize, cursor_x: usize, cursor_y: usize) -> (usize, usize) {
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

fn is_highlighted(x: usize, y: usize, visual_x: usize, visual_y: usize, cursor_x: usize, cursor_y: usize) -> bool {
    y < visual_y && y > cursor_y || y > visual_y && y < cursor_y
    || (y == cursor_y && y == visual_y && visual_y >= cursor_y && x >= visual_x && x <= cursor_x)
    || (y == cursor_y && y == visual_y && visual_y <= cursor_y && x <= visual_x && x >= cursor_x)
    || (y == cursor_y && cursor_y < visual_y && x >= cursor_x)
    || (y == cursor_y && cursor_y > visual_y && x <= cursor_x)
    || (y == visual_y && cursor_y < visual_y && x <= visual_x)
    || (y == visual_y && cursor_y > visual_y && x >= visual_x)
}

fn is_line_highlighted(y: usize, visual_y: usize, cursor_y: usize) -> bool {
    y <= visual_y && y >= cursor_y || y >= visual_y && y <= cursor_y
}

fn get_cursor_after_visual(cursor: usize, visual: usize) -> usize {
    if cursor <= visual {
        cursor
    } else {
        visual
    }
}

fn paste_before(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize) {
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

fn paste_after(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize) {
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

fn copy_in_visual(
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

fn delete_in_visual(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize, visual_x: usize, visual_y: usize, mode: char) -> (usize, usize) {
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

fn delete_in_visual_and_insert(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize) {
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

fn reset_cursor_end_file(length: usize, cursor_y: usize) -> usize {
    if cursor_y >= length {
        length - 1
    } else {
        cursor_y
    }
}

fn count_leading_spaces(input: &str) -> usize {
    let trimmed = input.trim_start();
    let leading_spaces = input.len() - trimmed.len();
    leading_spaces
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Please provide a file name");
        return;
    }
    let file_name = &args[1];
    enable_raw_mode().expect("Failed to enable raw mode");
    execute!(stdout(), EnterAlternateScreen).expect("Failed to enter alternate screen");
    let mut file_data = match get_file_data(file_name) {
        Ok(data) => data,
        Err(err) => {
            quit_terminal();
            println!("Failed to open the file: {}", err);
            return;
        }
    };
    let mut window_line_x = 0;
    let mut window_line_y = 0;
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut visual_x = 0;
    let mut visual_y = 0;
    let mut mode = 'n';
    let mut prev_keys = "";
    let mut prev_view: Vec<Vec<(char, Color, Color, bool)>> = Vec::new();
    prev_view = render_file_data(prev_view.clone(), &file_data, window_line_x, window_line_y, cursor_x, cursor_y, visual_x, visual_y, mode);
    loop {
        if let Ok(event) = crossterm::event::read() {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event {
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                } else {
                    if mode == 'n' {
                        if prev_keys == "r" {
                            if cursor_x < file_data[cursor_y].len() {
                                if let KeyCode::Char(c) = code {
                                    file_data[cursor_y].remove(cursor_x);
                                    file_data[cursor_y].insert(cursor_x, c);
                                    save_to_file(&file_data, file_name);
                                }
                            }
                            prev_keys = "";
                        } else if code == KeyCode::Char('h') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                            cursor_x = left(cursor_x);
                        } else if code == KeyCode::Char('l') {
                            cursor_x = right(&file_data, cursor_x, cursor_y);
                        } else if code == KeyCode::Char('j') {
                            cursor_y = down(&file_data, cursor_y);
                        } else if code == KeyCode::Char('k') {
                            cursor_y = up(cursor_y);
                        } else if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
                            save_to_file(&file_data, file_name);
                        } else if prev_keys == "d" && code == KeyCode::Char('i') {
                            // prev_keys += "i";
                        } else if prev_keys == "di" && code == KeyCode::Char('w') {
                            // TODO prev_keys += "i";
                        } else if code == KeyCode::Char('$') {
                            cursor_x = set_cursor_end(&file_data, cursor_y);
                            cursor_x = left(cursor_x);
                        } else if code == KeyCode::Char('^') {
                            cursor_x = count_leading_spaces(&file_data[cursor_y]);
                        } else if code == KeyCode::Char('0') {
                            cursor_x = 0;
                        } else if code == KeyCode::Char('a') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                            cursor_x = right(&file_data, cursor_x, cursor_y);
                            mode = 'i';
                        } else if code == KeyCode::Char('A') {
                            cursor_x = set_cursor_end(&file_data, cursor_y);
                            mode = 'i';
                        } else if code == KeyCode::Char('i') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                            mode = 'i';
                        } else if code == KeyCode::Char('I') {
                            cursor_x = count_leading_spaces(&file_data[cursor_y]);
                            mode = 'i';
                        } else if code == KeyCode::Char('>') {
                            file_data[cursor_y] = increase_indent(file_data[cursor_y].clone());
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('<') {
                            file_data[cursor_y] = reduce_indent(file_data[cursor_y].clone());
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('o') {
                            let mut indent_level = count_leading_spaces(&file_data[cursor_y]);
                            if file_data[cursor_y].ends_with('(') || file_data[cursor_y].ends_with('{') {
                                indent_level += 4;
                            }
                            file_data.insert(cursor_y + 1, " ".repeat(indent_level).to_string());
                            cursor_x = indent_level;
                            cursor_y = down(&file_data, cursor_y);
                            mode = 'i';
                        } else if code == KeyCode::Char('O') {
                            let mut indent_level = count_leading_spaces(&file_data[cursor_y]);
                            if file_data[cursor_y].ends_with('(') || file_data[cursor_y].ends_with('{') {
                                indent_level += 4;
                            }
                            cursor_x = indent_level;
                            file_data.insert(cursor_y, " ".repeat(indent_level).to_string());
                            mode = 'i';
                        } else if code == KeyCode::Char('v') {
                            mode = 'v';
                            visual_x = cursor_x;
                            visual_y = cursor_y;
                        } else if code == KeyCode::Char('V') {
                            mode = 'V';
                            visual_x = cursor_x;
                            visual_y = cursor_y;
                        } else if prev_keys == "g" && code == KeyCode::Char('g') {
                            cursor_y = 0;
                            prev_keys = "";
                        } else if code == KeyCode::Char('P') {
                            paste_before(&mut file_data, cursor_x, cursor_y);
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('p') {
                            cursor_x = prevent_cursor_end(&file_data, cursor_x, cursor_y);
                            paste_after(&mut file_data, cursor_x, cursor_y);
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('s') {
                            file_data[cursor_y].remove(cursor_x);
                            mode = 'i';
                        } else if code == KeyCode::Char('x') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                            if cursor_x < file_data[cursor_y].len() {
                                copy_to_clipboard(&file_data[cursor_y][cursor_x..cursor_x + 1]).expect("Failed to copy to clipboard");
                                file_data[cursor_y].remove(cursor_x);
                                save_to_file(&file_data, file_name);
                            }
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                        } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
                            let terminal_size = size().unwrap();
                            let term_height = terminal_size.1 as usize;
                            let mut i = 0;
                            while i < term_height {
                                cursor_y = down(&file_data, cursor_y);
                                i += 2;
                            }
                        } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
                            let terminal_size = size().unwrap();
                            let term_height = terminal_size.1 as usize;
                            let mut i = 0;
                            while i < term_height {
                                cursor_y = up(cursor_y);
                                i += 2;
                            }
                        } else if prev_keys == "c" && code == KeyCode::Char('c') {
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                            delete_in_visual_and_insert(&mut file_data, cursor_y, cursor_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'i';
                            prev_keys = "";
                        } else if prev_keys == "y" && code == KeyCode::Char('y') {
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                            prev_keys = "";
                        } else if prev_keys == "d" && code == KeyCode::Char('d') {
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                            delete_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            prev_keys = "";
                            save_to_file(&file_data, file_name);
                        } else if prev_keys == "" && code == KeyCode::Char('g') {
                            prev_keys = "g";
                        } else if prev_keys == "" && code == KeyCode::Char('r') {
                            prev_keys = "r";
                        } else if prev_keys == "" && code == KeyCode::Char('c') {
                            prev_keys = "c";
                        } else if prev_keys == "" && code == KeyCode::Char('d') {
                            prev_keys = "d";
                        } else if prev_keys == "" && code == KeyCode::Char('y') {
                            prev_keys = "y";
                        } else if code == KeyCode::Char('G') {
                            cursor_y = file_data.len() - 1;
                        } else if code == KeyCode::Esc {
                            prev_keys = "";
                            save_to_file(&file_data, file_name);
                        }
                    } else if mode == 'i' {
                        if code == KeyCode::Esc {
                            mode = 'n';
                            cursor_x = left(cursor_x);
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::BackTab {
                            file_data[cursor_y] = reduce_indent(file_data[cursor_y].clone());
                            if cursor_x >= 4 {
                                cursor_x -= 4;
                            } else {
                                cursor_x = 0;
                            }
                        } else if code == KeyCode::Tab {
                            file_data[cursor_y] = increase_indent(file_data[cursor_y].clone());
                            cursor_x += 4;
                        } else if code == KeyCode::Enter {
                            let mut indent_level = count_leading_spaces(&file_data[cursor_y]);
                            if file_data[cursor_y][..cursor_x].ends_with('(') || file_data[cursor_y][..cursor_x].ends_with('{') {
                                indent_level += 4;
                            }
                            let substring = " ".repeat(indent_level) + &file_data[cursor_y][cursor_x..];
                            if file_data[cursor_y][..cursor_x].ends_with('(') {
                                file_data.insert(cursor_y + 1, " ".repeat(indent_level - 4) + ")");
                            }
                            if file_data[cursor_y][..cursor_x].ends_with('{') {
                                file_data.insert(cursor_y + 1, " ".repeat(indent_level - 4) + "}");
                            }
                            file_data.insert(cursor_y + 1, substring.to_string());
                            file_data[cursor_y] = file_data[cursor_y][..cursor_x].to_string();
                            cursor_y += 1;
                            cursor_x = indent_level;
                        } else if code == KeyCode::Backspace {
                            if cursor_x > 0 {
                                file_data[cursor_y].remove(cursor_x - 1);
                                cursor_x = left(cursor_x);
                            }
                        } else if code == KeyCode::Delete {
                            file_data[cursor_y].remove(cursor_x);
                        } else if let KeyCode::Char(c) = code {
                            file_data[cursor_y].insert(cursor_x, c);
                            cursor_x += 1;
                        }
                    } else if mode == 'v' {
                        if code == KeyCode::Esc {
                            mode = 'n';
                        } else if code == KeyCode::Char('h') {
                            cursor_x = left(cursor_x);
                        } else if code == KeyCode::Char('l') {
                            cursor_x = right(&file_data, cursor_x, cursor_y);
                        } else if code == KeyCode::Char('j') {
                            cursor_y = down(&file_data, cursor_y);
                        } else if code == KeyCode::Char('k') {
                            cursor_y = up(cursor_y);
                        } else if prev_keys == "g" && code == KeyCode::Char('g') {
                            cursor_y = 0;
                            prev_keys = "";
                        } else if prev_keys == "" && code == KeyCode::Char('g') {
                            prev_keys = "g";
                        } else if code == KeyCode::Char('G') {
                            cursor_y = file_data.len() - 1;
                        } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
                            let terminal_size = size().unwrap();
                            let term_height = terminal_size.1 as usize;
                            let mut i = 0;
                            while i < term_height {
                                cursor_y = down(&file_data, cursor_y);
                                i += 2;
                            }
                        } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
                            let terminal_size = size().unwrap();
                            let term_height = terminal_size.1 as usize;
                            let mut i = 0;
                            while i < term_height {
                                cursor_y = up(cursor_y);
                                i += 2;
                            }
                        } else if code == KeyCode::Char('y') {
                            cursor_x = prevent_cursor_end(&file_data, cursor_x, cursor_y);
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_x = get_cursor_after_visual(cursor_x, visual_x);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('c') {
                            cursor_x = prevent_cursor_end(&file_data, cursor_x, cursor_y);
                            (cursor_x, cursor_y) = delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'i';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('d') {
                            cursor_x = prevent_cursor_end(&file_data, cursor_x, cursor_y);
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            (cursor_x, cursor_y) = delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('x') {
                            (cursor_x, cursor_y) = delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        }
                    } else if mode == 'V' {
                        if code == KeyCode::Esc {
                            mode = 'n';
                        } else if code == KeyCode::Char('j') {
                            cursor_y = down(&file_data, cursor_y);
                        } else if code == KeyCode::Char('k') {
                            cursor_y = up(cursor_y);
                        } else if prev_keys == "g" && code == KeyCode::Char('g') {
                            cursor_y = 0;
                            prev_keys = "";
                        } else if prev_keys == "" && code == KeyCode::Char('g') {
                            prev_keys = "g";
                        } else if code == KeyCode::Char('G') {
                            cursor_y = file_data.len() - 1;
                        } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
                            let terminal_size = size().unwrap();
                            let term_height = terminal_size.1 as usize;
                            let mut i = 0;
                            while i < term_height {
                                cursor_y = down(&file_data, cursor_y);
                                i += 2;
                            }
                        } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
                            let terminal_size = size().unwrap();
                            let term_height = terminal_size.1 as usize;
                            let mut i = 0;
                            while i < term_height {
                                cursor_y = up(cursor_y);
                                i += 2;
                            }
                        } else if code == KeyCode::Char('>') {
                            increase_indent_visual(&mut file_data, cursor_y, visual_y);
                            save_to_file(&file_data, file_name);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            mode = 'n';
                        } else if code == KeyCode::Char('<') {
                            reduce_indent_visual(&mut file_data, cursor_y, visual_y);
                            save_to_file(&file_data, file_name);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            mode = 'n';
                        } else if code == KeyCode::Char('y') {
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('c') {
                            delete_in_visual_and_insert(&mut file_data, cursor_y, visual_y);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'i';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('d') {
                            copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            (cursor_x, cursor_y) = delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('x') {
                            (cursor_x, cursor_y) = delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        }
                    }
                    (window_line_x, window_line_y) = calc_window_lines(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
                    prev_view = render_file_data(prev_view.clone(), &file_data, window_line_x, window_line_y, cursor_x, cursor_y, visual_x, visual_y, mode);
                }
            }
        }
    }
    quit_terminal();
}
