use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::terminal::{Clear, ClearType};
use crossterm::cursor::{MoveToColumn, MoveToRow};
use crossterm::style::{Color, Print, SetForegroundColor};
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader};
use crossterm::terminal::size;

fn get_file_data(file_name: &str) -> io::Result<Vec<String>> {
    let file_path = format!("{}", file_name);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    Ok(lines)
}

fn render_file_data(file_data: &[String], window_line_x: usize, window_line_y: usize, cursor_x: usize, cursor_y: usize) {
    let mut stdout = stdout();

    execute!(stdout, Clear(ClearType::All)).expect("Failed to clear screen");
    let terminal_size = size().unwrap();
    let term_height = terminal_size.1 as usize;
    let term_width = terminal_size.0 as usize;
    let mut y = 0;

    while y < term_height && y < file_data.len() {
        execute!(stdout, MoveToRow(((y + 1) as u16).try_into().unwrap())).expect("Failed to move cursor");
        let mut line: String = if file_data[window_line_y + y].len() >= window_line_x { file_data[window_line_y + y][window_line_x..].to_string() } else { "".to_string() };
        if line.len() > term_width {
            let substring = &line[..term_width];
            line = String::from(substring);
        }
        execute!(
            stdout,
            MoveToColumn(1),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{:4} ", window_line_y + y + 1)),
            SetForegroundColor(Color::White),
            Print(line)
        ).expect("Failed to execute command");
        y += 1;
    }
    execute!(stdout, MoveToRow(cursor_y as u16 + 1 - window_line_y as u16)).expect("Failed to move cursor");
    let cursor_x_display: u16 = if cursor_x > file_data[cursor_y].len() {
        file_data[cursor_y].len().try_into().unwrap()
    } else {
        cursor_x as u16 - window_line_x as u16
    };
    execute!(stdout, MoveToColumn(cursor_x_display as u16 + 6)).expect("Failed to move cursor");

}

fn quit_terminal() {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen).expect("Failed to leave alternate screen");
    disable_raw_mode().expect("Failed to disable raw mode");
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
            println!("Failed to open the file: {}", err);
            quit_terminal();
            return;
        }
    };
    let mut window_line_x = 0;
    let mut window_line_y = 0;
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut mode = 'n';
    render_file_data(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
    loop {
        if let Ok(event) = crossterm::event::read() {
            if let Event::Key(KeyEvent { code, modifiers }) = event {
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                } else {
                    if mode == 'n' {
                        if code == KeyCode::Char('h') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                            cursor_x = left(cursor_x);
                        } else if code == KeyCode::Char('l') {
                            cursor_x = right(&file_data, cursor_x, cursor_y);
                        } else if code == KeyCode::Char('j') {
                            cursor_y = down(&file_data, cursor_y);
                        } else if code == KeyCode::Char('k') {
                            cursor_y = up(cursor_y);
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
                        }
                    } else if mode == 'i' {
                        if code == KeyCode::Esc {
                            mode = 'n';
                            cursor_x = left(cursor_x);
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
                    }
                    (window_line_x, window_line_y) = calc_window_lines(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
                    render_file_data(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
                }
            }
        }
    }
    quit_terminal();
}
