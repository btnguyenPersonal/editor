use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::cursor::{MoveLeft, MoveRight, MoveUp, MoveDown};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::terminal::{Clear, ClearType};
use crossterm::cursor::MoveToColumn;
use crossterm::style::{Print};
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader};

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

    for line in file_data {
        execute!(stdout, MoveToColumn(0), Print(line), Print("\n")).expect("Failed to execute command");
    }
    execute!(stdout, MoveUp(file_data.len() as u16 - cursor_y as u16)).expect("Failed to move cursor");
    execute!(stdout, MoveToColumn(cursor_x as u16)).expect("Failed to move cursor");
}

fn quit_terminal() {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen).expect("Failed to leave alternate screen");
    disable_raw_mode().expect("Failed to disable raw mode");
}

fn main() {
    // Get the command-line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if a file name argument was provided
    if args.len() < 2 {
        println!("Please provide a file name");
        return;
    }

    // Get the file name from the arguments
    let file_name = &args[1];

    // Enable raw mode and enter alternate screen
    enable_raw_mode().expect("Failed to enable raw mode");
    execute!(stdout(), EnterAlternateScreen).expect("Failed to enter alternate screen");

    // Render file data
    let file_data = match get_file_data(file_name) {
        Ok(data) => data,
        Err(err) => {
            println!("Failed to open the file: {}", err);
            quit_terminal();
            return;
        }
    };
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    render_file_data(&file_data, cursor_x, cursor_y);

    // Wait for Ctrl+C to exit
    loop {
        if let Ok(event) = crossterm::event::read() {
            if let Event::Key(KeyEvent { code, .. }) = event {
                match code {
                    KeyCode::Char('c') => break,
                    KeyCode::Char('h') => {
                        if cursor_x > 0 {
                            cursor_x -= 1;
                        }
                    }
                    KeyCode::Char('l') => {
                        if cursor_x < file_data[cursor_y].len() {
                            cursor_x += 1;
                        }
                    }
                    KeyCode::Char('k') => {
                        if cursor_y > 0 {
                            cursor_y -= 1;
                        }
                    }
                    KeyCode::Char('j') => {
                        if cursor_y < file_data.len() - 1 {
                            cursor_y += 1;
                        }
                    }
                    _ => {}
                }

                render_file_data(&file_data, cursor_x, cursor_y);
            }
        }
    }

    // Quit the terminal
    quit_terminal();
}
