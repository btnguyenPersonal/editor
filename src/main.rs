use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
use crossterm::style::{Color};
use std::io::{stdout};
use crossterm::terminal::size;
use std::panic;
mod helper;

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        helper::quit_terminal();
        if let Some(location) = panic_info.location() {
            eprintln!("Panic occurred at {}:{}\n{}", location.file(), location.line(), panic_info);
        } else {
            eprintln!("Panic occurred at unknown location\n{}", panic_info);
        }
    }));
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Please provide a file name");
        return;
    }
    let file_name = &args[1];
    enable_raw_mode().expect("Failed to enable raw mode");
    execute!(stdout(), EnterAlternateScreen).expect("Failed to enter alternate screen");
    let mut file_data = match helper::get_file_data(file_name) {
        Ok(data) => data,
        Err(err) => {
            helper::quit_terminal();
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
    prev_view = helper::render_file_data(prev_view.clone(), &file_data, window_line_x, window_line_y, cursor_x, cursor_y, visual_x, visual_y, mode);
    loop {
        if let Ok(event) = crossterm::event::read() {
            let Event::Key(KeyEvent { code, modifiers, .. }) = event else { break; };
            if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                break;
            } else {
                if mode == 'n' {
                    if prev_keys == "r" {
                        if cursor_x < file_data[cursor_y].len() {
                            if let KeyCode::Char(c) = code {
                                file_data.get_mut(cursor_y).expect(&format!("failed trying to access {}", cursor_y)).remove(cursor_x);
                                file_data.get_mut(cursor_y).expect(&format!("failed trying to access {}", cursor_y)).insert(cursor_x, c);
                                helper::save_to_file(&file_data, file_name);
                            }
                        }
                        prev_keys = "";
                    } else if prev_keys == "d" && code == KeyCode::Char('i') {
                        prev_keys = "di";
                    } else if prev_keys == "di" && code == KeyCode::Char('w') {
                        cursor_x = helper::get_index_next_word(&file_data, cursor_x, cursor_y);
                        prev_keys = "";
                    } else if code == KeyCode::Char('h') {
                        cursor_x = helper::reset_cursor_end(&file_data, cursor_x, cursor_y);
                        cursor_x = helper::left(cursor_x);
                    } else if code == KeyCode::Char('l') {
                        cursor_x = helper::right(&file_data, cursor_x, cursor_y);
                    } else if code == KeyCode::Char('j') {
                        cursor_y = helper::down(&file_data, cursor_y);
                    } else if code == KeyCode::Char('k') {
                        cursor_y = helper::up(cursor_y);
                    } else if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('$') {
                        cursor_x = helper::set_cursor_end(&file_data, cursor_y);
                        cursor_x = helper::left(cursor_x);
                    } else if code == KeyCode::Char('^') {
                        cursor_x = helper::count_leading_spaces(&file_data[cursor_y]);
                    } else if code == KeyCode::Char('0') {
                        cursor_x = 0;
                    } else if code == KeyCode::Char('b') {
                        cursor_x = helper::get_index_prev_word(&file_data, cursor_x, cursor_y);
                    } else if code == KeyCode::Char('w') {
                        cursor_x = helper::get_index_next_word(&file_data, cursor_x, cursor_y);
                    } else if code == KeyCode::Char('a') {
                        cursor_x = helper::reset_cursor_end(&file_data, cursor_x, cursor_y);
                        cursor_x = helper::right(&file_data, cursor_x, cursor_y);
                        mode = 'i';
                    } else if code == KeyCode::Char('A') {
                        cursor_x = helper::set_cursor_end(&file_data, cursor_y);
                        mode = 'i';
                    } else if code == KeyCode::Char('i') {
                        cursor_x = helper::reset_cursor_end(&file_data, cursor_x, cursor_y);
                        mode = 'i';
                    } else if code == KeyCode::Char('I') {
                        cursor_x = helper::count_leading_spaces(&file_data[cursor_y]);
                        mode = 'i';
                    } else if code == KeyCode::Char('>') {
                        file_data[cursor_y] = helper::increase_indent(file_data[cursor_y].clone());
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('<') {
                        file_data[cursor_y] = helper::reduce_indent(file_data[cursor_y].clone());
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('o') {
                        let mut indent_level = helper::count_leading_spaces(&file_data[cursor_y]);
                        if file_data[cursor_y].ends_with('(') || file_data[cursor_y].ends_with('{') {
                            indent_level += 4;
                        }
                        file_data.insert(cursor_y + 1, " ".repeat(indent_level).to_string());
                        cursor_x = indent_level;
                        cursor_y = helper::down(&file_data, cursor_y);
                        mode = 'i';
                    } else if code == KeyCode::Char('O') {
                        let mut indent_level = helper::count_leading_spaces(&file_data[cursor_y]);
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
                        helper::paste_before(&mut file_data, cursor_x, cursor_y);
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('p') {
                        cursor_x = helper::prevent_cursor_end(&file_data, cursor_x, cursor_y);
                        helper::paste_after(&mut file_data, cursor_x, cursor_y);
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('s') {
                        file_data[cursor_y].remove(cursor_x);
                        mode = 'i';
                    } else if code == KeyCode::Char('x') {
                        cursor_x = helper::reset_cursor_end(&file_data, cursor_x, cursor_y);
                        if cursor_x < file_data[cursor_y].len() {
                            helper::copy_to_clipboard(&file_data[cursor_y][cursor_x..cursor_x + 1]).expect("Failed to copy to clipboard");
                            file_data[cursor_y].remove(cursor_x);
                            helper::save_to_file(&file_data, file_name);
                        }
                        cursor_x = helper::reset_cursor_end(&file_data, cursor_x, cursor_y);
                    } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
                        let terminal_size = size().unwrap();
                        let term_height = terminal_size.1 as usize;
                        let mut i = 0;
                        while i < term_height {
                            cursor_y = helper::down(&file_data, cursor_y);
                            i += 2;
                        }
                    } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
                        let terminal_size = size().unwrap();
                        let term_height = terminal_size.1 as usize;
                        let mut i = 0;
                        while i < term_height {
                            cursor_y = helper::up(cursor_y);
                            i += 2;
                        }
                    } else if prev_keys == "c" && code == KeyCode::Char('c') {
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                        helper::delete_in_visual_and_insert(&mut file_data, cursor_y, cursor_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'i';
                        prev_keys = "";
                    } else if prev_keys == "y" && code == KeyCode::Char('y') {
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                        prev_keys = "";
                    } else if prev_keys == "d" && code == KeyCode::Char('d') {
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                        helper::delete_in_visual(&mut file_data, cursor_x, cursor_y, cursor_x, cursor_y, 'V');
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        prev_keys = "";
                        helper::save_to_file(&file_data, file_name);
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
                        helper::save_to_file(&file_data, file_name);
                    }
                } else if mode == 'i' {
                    if code == KeyCode::Esc {
                        mode = 'n';
                        cursor_x = helper::left(cursor_x);
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::BackTab {
                        file_data[cursor_y] = helper::reduce_indent(file_data[cursor_y].clone());
                        if cursor_x >= 4 {
                            cursor_x -= 4;
                        } else {
                            cursor_x = 0;
                        }
                    } else if code == KeyCode::Tab {
                        file_data[cursor_y] = helper::increase_indent(file_data[cursor_y].clone());
                        cursor_x += 4;
                    } else if code == KeyCode::Enter {
                        let mut indent_level = helper::count_leading_spaces(&file_data[cursor_y]);
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
                            cursor_x = helper::left(cursor_x);
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
                        cursor_x = helper::left(cursor_x);
                    } else if code == KeyCode::Char('l') {
                        cursor_x = helper::right(&file_data, cursor_x, cursor_y);
                    } else if code == KeyCode::Char('j') {
                        cursor_y = helper::down(&file_data, cursor_y);
                    } else if code == KeyCode::Char('k') {
                        cursor_y = helper::up(cursor_y);
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
                            cursor_y = helper::down(&file_data, cursor_y);
                            i += 2;
                        }
                    } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
                        let terminal_size = size().unwrap();
                        let term_height = terminal_size.1 as usize;
                        let mut i = 0;
                        while i < term_height {
                            cursor_y = helper::up(cursor_y);
                            i += 2;
                        }
                    } else if code == KeyCode::Char('y') {
                        cursor_x = helper::prevent_cursor_end(&file_data, cursor_x, cursor_y);
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_x = helper::get_cursor_after_visual(cursor_x, visual_x);
                        mode = 'n';
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('c') {
                        cursor_x = helper::prevent_cursor_end(&file_data, cursor_x, cursor_y);
                        (cursor_x, cursor_y) = helper::delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'i';
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('d') {
                        cursor_x = helper::prevent_cursor_end(&file_data, cursor_x, cursor_y);
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        (cursor_x, cursor_y) = helper::delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'n';
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('x') {
                        (cursor_x, cursor_y) = helper::delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'n';
                        helper::save_to_file(&file_data, file_name);
                    }
                } else if mode == 'V' {
                    if code == KeyCode::Esc {
                        mode = 'n';
                    } else if code == KeyCode::Char('j') {
                        cursor_y = helper::down(&file_data, cursor_y);
                    } else if code == KeyCode::Char('k') {
                        cursor_y = helper::up(cursor_y);
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
                            cursor_y = helper::down(&file_data, cursor_y);
                            i += 2;
                        }
                    } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
                        let terminal_size = size().unwrap();
                        let term_height = terminal_size.1 as usize;
                        let mut i = 0;
                        while i < term_height {
                            cursor_y = helper::up(cursor_y);
                            i += 2;
                        }
                    } else if code == KeyCode::Char('>') {
                        helper::increase_indent_visual(&mut file_data, cursor_y, visual_y);
                        helper::save_to_file(&file_data, file_name);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        mode = 'n';
                    } else if code == KeyCode::Char('<') {
                        helper::reduce_indent_visual(&mut file_data, cursor_y, visual_y);
                        helper::save_to_file(&file_data, file_name);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        mode = 'n';
                    } else if code == KeyCode::Char('y') {
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        mode = 'n';
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('c') {
                        helper::delete_in_visual_and_insert(&mut file_data, cursor_y, visual_y);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'i';
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('d') {
                        helper::copy_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        (cursor_x, cursor_y) = helper::delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'n';
                        helper::save_to_file(&file_data, file_name);
                    } else if code == KeyCode::Char('x') {
                        (cursor_x, cursor_y) = helper::delete_in_visual(&mut file_data, cursor_x, cursor_y, visual_x, visual_y, mode);
                        cursor_y = helper::get_cursor_after_visual(cursor_y, visual_y);
                        cursor_y = helper::reset_cursor_end_file(file_data.len(), cursor_y);
                        mode = 'n';
                        helper::save_to_file(&file_data, file_name);
                    }
                }
                (window_line_x, window_line_y) = helper::calc_window_lines(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
                prev_view = helper::render_file_data(prev_view.clone(), &file_data, window_line_x, window_line_y, cursor_x, cursor_y, visual_x, visual_y, mode);
            }
        }
    }
    helper::quit_terminal();
}
