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

fn render_file_data(file_data: &[String], cursor_x: usize, cursor_y: usize) {
    let mut stdout = stdout();

    execute!(stdout, Clear(ClearType::All)).expect("Failed to clear screen");
    let terminal_size = size().unwrap();
    let term_height = terminal_size.1 as usize;
    let mut y = 0;

    execute!(stdout, MoveToRow(0)).expect("Failed to move cursor");
    while y <= term_height && y < file_data.len() {
        let line: &String = &file_data[y];
        execute!(
            stdout,
            MoveToColumn(0),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{:5} ", y + 1)),
            SetForegroundColor(Color::White),
            Print(line),
            Print("\n")
        ).expect("Failed to execute command");
        y += 1;
    }
    execute!(stdout, MoveToRow(cursor_y as u16 + 1)).expect("Failed to move cursor");
    let cursor_x_display: u16 = if cursor_x > file_data[cursor_y].len() {
        file_data[cursor_y].len().try_into().unwrap()
    } else {
        cursor_x.try_into().unwrap()
    };
    execute!(stdout, MoveToColumn(cursor_x_display as u16 + 7)).expect("Failed to move cursor");

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

fn reset_cursor_end(file_data: &[String], cursor_x: usize, cursor_y: usize) -> usize {
    if cursor_x > file_data[cursor_y].len() {
        file_data[cursor_y].len()
    } else {
        cursor_x
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
            println!("Failed to open the file: {}", err);
            quit_terminal();
            return;
        }
    };
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut mode = 'n';
    render_file_data(&file_data, cursor_x, cursor_y);
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
                        } else if code == KeyCode::Char('a') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
                            cursor_x = right(&file_data, cursor_x, cursor_y);
                            mode = 'i';
                        } else if code == KeyCode::Char('i') {
                            cursor_x = reset_cursor_end(&file_data, cursor_x, cursor_y);
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
                            if c.len_utf8() == 1 {
                                file_data[cursor_y].insert(cursor_x, c);
                                cursor_x += 1;
                            }
                        }
                    }
                    render_file_data(&file_data, cursor_x, cursor_y);
                }
            }
        }
    }
    quit_terminal();
}
