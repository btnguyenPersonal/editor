use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
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

fn render_file_data(file_data: &[String]) {
    let mut stdout = stdout();

    for line in file_data {
        execute!(stdout, MoveToColumn(0), Print(line), Print("\n")).expect("Failed to execute command");
    }
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
    print!("{:#?}", file_data);
    render_file_data(&file_data);

    // Wait for Ctrl+C to exit
    loop {
        print!("hi");
        if let Ok(event) = crossterm::event::read() {
            if let Event::Key(KeyEvent { code, .. }) = event {
                if code == KeyCode::Char('c') {
                    break;
                }
            }
        }
    }

    // Quit the terminal
    quit_terminal();
}
