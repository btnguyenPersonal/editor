use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::terminal::{Clear, ClearType};
use crossterm::cursor::{Hide, Show, MoveToColumn, MoveToRow};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor, SetAttribute, Attribute};
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader, Write};
use crossterm::terminal::size;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

fn copy_to_clipboard(text: &str) {
    let mut clipboard: ClipboardContext = ClipboardProvider::new().expect("Failed to create clipboard context");
    let _ = clipboard.set_contents(text.to_owned());
}

fn get_file_data(file_name: &str) -> io::Result<Vec<String>> {
    let file_path = format!("{}", file_name);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    Ok(lines)
}

fn render_file_data(file_data: &[String], window_line_x: usize, window_line_y: usize, cursor_x: usize, cursor_y: usize, visual_x: usize, visual_y: usize, mode: char) {
    let mut stdout = stdout();
    execute!(stdout, Hide);
    let terminal_size = size().unwrap();
    let term_height = terminal_size.1 as usize;
    let term_width = terminal_size.0 as usize;
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
        execute!(
            stdout,
            MoveToColumn(0),
            ResetColor,
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{:4} ", window_line_y + y + 1)),
        ).expect("Failed to print line number");
        if is_line_highlighted(window_line_y + y, visual_y, cursor_y, mode) {
            execute!(
                stdout,
                SetAttribute(Attribute::Reverse)
            );
        }
        let mut x = 0;
        while x < line.len() {
            execute!(
                stdout,
                SetForegroundColor(Color::White),
                Print(&line[x..x+1])
            );
            x += 1;
        }
        while x < term_width - 5 {
            execute!(
                stdout,
                SetForegroundColor(Color::White),
                Print(" ")
            );
            x += 1;
        }
        y += 1;
    }
    execute!(stdout, MoveToRow(cursor_y as u16 - window_line_y as u16)).expect("Failed to move cursor");
    let cursor_x_display: u16 = if cursor_x > file_data[cursor_y].len() {
        file_data[cursor_y].len().try_into().unwrap()
    } else {
        cursor_x as u16 - window_line_x as u16
    };
    execute!(stdout, MoveToColumn(cursor_x_display as u16 + 5)).expect("Failed to move cursor");
    execute!(stdout, Show);
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
    if cursor_x < file_data[cursor_y].len() {
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

fn reset_cursor_end(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if cursor_x > file_data[cursor_y].len() {
        file_data[cursor_y].len()
    } else {
        cursor_x
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

fn is_line_highlighted(y: usize, visual_y: usize, cursor_y: usize, mode: char) -> bool {
    mode == 'V' && (y <= visual_y && y >= cursor_y || y >= visual_y && y <= cursor_y)
}

fn get_cursor_after_visual(cursor: usize, visual: usize) -> usize {
    if cursor <= visual {
        cursor
    } else {
        visual
    }
}

fn get_clipboard_content() -> String {
    let mut clipboard: ClipboardContext = ClipboardProvider::new().ok().expect("clipboard retrieval error");
    clipboard.get_contents().ok().expect("clipboard retreival error")
}

fn paste_before(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize) {
    let mut clip = get_clipboard_content();
    if clip.starts_with("\n") {
        clip.remove(0);
        let lines: Vec<&str> = clip.split('\n').collect();
        for line in lines {
            let _ = &file_data.insert(cursor_y, line.to_string());
        }
    }
}

fn paste_after(file_data: &mut Vec<String>, cursor_x: usize, cursor_y: usize) {
    let mut clip = get_clipboard_content();
    if clip.starts_with("\n") {
        clip.remove(0);
        let lines: Vec<&str> = clip.split('\n').collect();
        for line in lines {
            let _ = &file_data.insert(cursor_y + 1, line.to_string());
        }
    }
}

fn copy_in_visual(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize, mode: char) {
    let mut clipboard: String = if mode == 'V' {"\n".to_string()} else {"".to_string()};
    let (begin, end) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for i in begin..end {
        clipboard += &file_data[i];
        clipboard += "\n";
    }
    clipboard += &file_data[end];
    copy_to_clipboard(&clipboard);
}

fn delete_in_visual(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize, mode: char) {
    let (begin, end) = if cursor_y <= visual_y {
        (cursor_y, visual_y)
    } else {
        (visual_y, cursor_y)
    };
    for _ in begin..=end {
        file_data.remove(begin);
    }
    if file_data.len() == 0 {
        file_data.insert(0, "".to_string());
    }
}

fn delete_in_visual_and_insert(file_data: &mut Vec<String>, cursor_y: usize, visual_y: usize, mode: char) {
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
    render_file_data(&file_data, window_line_x, window_line_y, cursor_x, cursor_y, visual_x, visual_y, mode);
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
                            paste_after(&mut file_data, cursor_x, cursor_y);
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('s') {
                            file_data[cursor_y].remove(cursor_x);
                            mode = 'i';
                        } else if code == KeyCode::Char('x') {
                            if cursor_x < file_data[cursor_y].len() {
                                copy_to_clipboard(&file_data[cursor_y][cursor_x..cursor_x + 1]);
                                file_data[cursor_y].remove(cursor_x);
                                save_to_file(&file_data, file_name);
                            }
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
                            copy_in_visual(&mut file_data, cursor_y, cursor_y, 'V');
                            delete_in_visual_and_insert(&mut file_data, cursor_y, cursor_y, 'V');
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'i';
                            prev_keys = "";
                        } else if prev_keys == "y" && code == KeyCode::Char('y') {
                            copy_in_visual(&mut file_data, cursor_y, cursor_y, 'V');
                            prev_keys = "";
                        } else if prev_keys == "d" && code == KeyCode::Char('d') {
                            copy_in_visual(&mut file_data, cursor_y, cursor_y, 'V');
                            delete_in_visual(&mut file_data, cursor_y, cursor_y, 'V');
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
                    } else if mode == 'V' {
                        if code == KeyCode::Esc {
                            mode = 'n';
                        } else if code == KeyCode::Char('j') {
                            cursor_y = down(&file_data, cursor_y);
                        } else if code == KeyCode::Char('k') {
                            cursor_y = up(cursor_y);
                        } else if code == KeyCode::Char('y') {
                            copy_in_visual(&mut file_data, cursor_y, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('c') {
                            delete_in_visual_and_insert(&mut file_data, cursor_y, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'i';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('d') {
                            copy_in_visual(&mut file_data, cursor_y, visual_y, mode);
                            delete_in_visual(&mut file_data, cursor_y, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        } else if code == KeyCode::Char('x') {
                            delete_in_visual(&mut file_data, cursor_y, visual_y, mode);
                            cursor_y = get_cursor_after_visual(cursor_y, visual_y);
                            cursor_y = reset_cursor_end_file(file_data.len(), cursor_y);
                            mode = 'n';
                            save_to_file(&file_data, file_name);
                        }
                    }
                    (window_line_x, window_line_y) = calc_window_lines(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
                    render_file_data(&file_data, window_line_x, window_line_y, cursor_x, cursor_y, visual_x, visual_y, mode);
                }
            }
        }
    }
    quit_terminal();
}
