use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
use crossterm::style::{Color};
use std::io::{stdout};
use crossterm::terminal::size;
use std::panic;
mod diffhist;
mod helper;

fn send_command(
    code: KeyCode,
    modifiers: KeyModifiers,
    file_data: &mut Vec<String>,
    file_name: &String,
    cursor_x: &mut usize,
    cursor_y: &mut usize,
    visual_x: &mut usize,
    visual_y: &mut usize,
    mode: &mut char,
    prev_keys: &mut String,
    last_command: &mut Vec<(KeyCode, KeyModifiers)>,
    recording: &mut bool,
    search_string: &mut String,
    searching: &mut bool,
    macro_command: &mut Vec<(KeyCode, KeyModifiers)>,
    macro_recording: &mut bool,
    diff_history: &mut diffhist::DiffHistory,
    pos: &mut (usize, usize),
    ) {
    if *mode == 'n' {
        *pos = (*cursor_x, *cursor_y);
        *searching = false;
        if prev_keys == "r" {
            if *cursor_x < file_data[*cursor_y].len() {
                if let KeyCode::Char(c) = code {
                    file_data[*cursor_y].remove(*cursor_x);
                    file_data[*cursor_y].insert(*cursor_x, c);
                    helper::log_command(code, modifiers, last_command, *recording);
                    helper::save_to_file(file_data, file_name, diff_history, *pos);
                }
            }
            *prev_keys = "".to_string();
        } else if code == KeyCode::Char(',') {
            *recording = false;
            for (macro_code, macro_modifiers) in macro_command.iter() {
                send_command(
                    *macro_code,
                    *macro_modifiers,
                    file_data,
                    file_name,
                    cursor_x,
                    cursor_y,
                    visual_x,
                    visual_y,
                    mode,
                    prev_keys,
                    last_command,
                    recording,
                    search_string,
                    searching,
                    &mut Vec::new(),
                    macro_recording,
                    diff_history,
                    pos,
                );
            }
            *recording = true;
        } else if code == KeyCode::Char('.') {
            *recording = false;
            for (last_code, last_modifiers) in last_command.iter() {
                send_command(
                    *last_code,
                    *last_modifiers,
                    file_data,
                    file_name,
                    cursor_x,
                    cursor_y,
                    visual_x,
                    visual_y,
                    mode,
                    prev_keys,
                    &mut Vec::new(),
                    recording,
                    search_string,
                    searching,
                    macro_command,
                    macro_recording,
                    diff_history,
                    pos,
                );
            }
            *recording = true;
        } else if code == KeyCode::Char('q') {
            if !*macro_recording {
                macro_command.clear();
            }
            *macro_recording = !*macro_recording;
        } else if code == KeyCode::Char('u') {
            match diff_history.undo() {
                Some((prev_state, (x, y))) => { *file_data = prev_state; *cursor_x = x; *cursor_y = y },
                None => ()
            }
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            if file_data.len() == 0 {
                file_data.insert(0, "".to_string());
            }
            helper::save_to_file_no_snapshot(file_data, file_name);
        } else if code == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL) {
            match diff_history.redo() {
                Some((next_state, (x, y))) => { *file_data = next_state; *cursor_x = x; *cursor_y = y },
                None => ()
            }
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            if file_data.len() == 0 {
                file_data.insert(0, "".to_string());
            }
            helper::save_to_file_no_snapshot(file_data, file_name);
        } else if *prev_keys == "y" && code == KeyCode::Char('i') {
            *prev_keys = "yi".to_string();
        } else if *prev_keys == "yi" && code == KeyCode::Char('w') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            match helper::get_in_word(&file_data[*cursor_y], *cursor_x) {
                Some((begin, end)) => {
                    let new_end = helper::prevent_cursor_end(&file_data, end, *cursor_y);
                    (*cursor_x, *cursor_y) = helper::copy_in_visual(file_data, begin, *cursor_y, new_end, *cursor_y, 'v');
                }
                None => ()
            };
            *prev_keys = "".to_string();
        } else if *prev_keys == "c" && code == KeyCode::Char('i') {
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "ci".to_string();
        } else if *prev_keys == "ci" && code == KeyCode::Char('w') {
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            match helper::get_in_word(&file_data[*cursor_y], *cursor_x) {
                Some((begin, end)) => {
                    let new_end = helper::prevent_cursor_end(&file_data, end, *cursor_y);
                    (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, begin, *cursor_y, new_end, *cursor_y, 'v');
                }
                None => ()
            };
            *mode = 'i';
            *prev_keys = "".to_string();
        } else if *prev_keys == "d" && code == KeyCode::Char('i') {
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "di".to_string();
        } else if *prev_keys == "di" && code == KeyCode::Char('w') {
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            match helper::get_in_word(&file_data[*cursor_y], *cursor_x) {
                Some((begin, end)) => {
                    let new_end = helper::prevent_cursor_end(&file_data, end, *cursor_y);
                    helper::copy_in_visual(file_data, begin, *cursor_y, new_end, *cursor_y, 'v');
                    (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, begin, *cursor_y, new_end, *cursor_y, 'v');
                }
                None => ()
            };
            helper::save_to_file(file_data, file_name, diff_history, *pos);
            *prev_keys = "".to_string();
        } else if code == KeyCode::Char('{') {
            *cursor_y = helper::get_prev_empty_line(&file_data, *cursor_y);
        } else if code == KeyCode::Char('}') {
            *cursor_y = helper::get_next_empty_line(&file_data, *cursor_y);
        } else if code == KeyCode::Char('h') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *cursor_x = helper::left(*cursor_x);
        } else if code == KeyCode::Char('l') {
            *cursor_x = helper::right(&file_data, *cursor_x, *cursor_y);
        } else if code == KeyCode::Char('j') {
            *cursor_y = helper::down(&file_data, *cursor_y);
        } else if code == KeyCode::Char('k') {
            *cursor_y = helper::up(*cursor_y);
        } else if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('$') {
            *cursor_x = helper::set_cursor_end(&file_data, *cursor_y);
            *cursor_x = helper::left(*cursor_x);
        } else if code == KeyCode::Char('^') {
            *cursor_x = helper::count_leading_spaces(&file_data[*cursor_y]);
        } else if code == KeyCode::Char('0') {
            *cursor_x = 0;
        } else if code == KeyCode::Char('b') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *cursor_x = helper::get_index_prev_word(&file_data, *cursor_x, *cursor_y);
        } else if code == KeyCode::Char('w') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *cursor_x = helper::get_index_next_word(&file_data, *cursor_x, *cursor_y);
        } else if code == KeyCode::Char('a') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *cursor_x = helper::right_insert(&file_data, *cursor_x, *cursor_y);
            *mode = 'i';
        } else if code == KeyCode::Char('A') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::set_cursor_end(&file_data, *cursor_y);
            *mode = 'i';
        } else if code == KeyCode::Char('i') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *mode = 'i';
        } else if code == KeyCode::Char('I') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::count_leading_spaces(&file_data[*cursor_y]);
            *mode = 'i';
        } else if code == KeyCode::Char('>') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            file_data[*cursor_y] = helper::increase_indent(file_data[*cursor_y].clone());
            *cursor_x = helper::count_leading_spaces(&file_data[*cursor_y]);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('<') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            file_data[*cursor_y] = helper::reduce_indent(file_data[*cursor_y].clone());
            *cursor_x = helper::count_leading_spaces(&file_data[*cursor_y]);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('o') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            let mut indent_level = helper::count_leading_spaces(&file_data[*cursor_y]);
            if file_data[*cursor_y].ends_with('(') || file_data[*cursor_y].ends_with('{') {
                indent_level += 4;
            }
            file_data.insert(*cursor_y + 1, " ".repeat(indent_level).to_string());
            *cursor_x = indent_level;
            *cursor_y = helper::down(&file_data, *cursor_y);
            *mode = 'i';
        } else if code == KeyCode::Char('O') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            let mut indent_level = helper::count_leading_spaces(&file_data[*cursor_y]);
            if file_data[*cursor_y].ends_with('(') || file_data[*cursor_y].ends_with('{') {
                indent_level += 4;
            }
            *cursor_x = indent_level;
            file_data.insert(*cursor_y, " ".repeat(indent_level).to_string());
            *mode = 'i';
        } else if code == KeyCode::Char('v') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *mode = 'v';
            *visual_x = *cursor_x;
            *visual_y = *cursor_y;
        } else if code == KeyCode::Char('V') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *mode = 'V';
            *visual_x = *cursor_x;
            *visual_y = *cursor_y;
        } else if *prev_keys == "g" && code == KeyCode::Char('g') {
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_y = 0;
            *prev_keys = "".to_string();
        } else if code == KeyCode::Char('P') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            helper::paste_before(file_data, *cursor_x, *cursor_y);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('p') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::prevent_cursor_end(&file_data, *cursor_x, *cursor_y);
            helper::paste_after(file_data, *cursor_x, *cursor_y);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('s') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            file_data[*cursor_y].remove(*cursor_x);
            *mode = 'i';
        } else if code == KeyCode::Char('x') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            if *cursor_x < file_data[*cursor_y].len() {
                helper::copy_to_clipboard(&file_data[*cursor_y][*cursor_x..*cursor_x + 1]).expect("Failed to copy to clipboard");
                file_data[*cursor_y].remove(*cursor_x);
                helper::save_to_file(file_data, file_name, diff_history, *pos);
            }
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
        } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
            let terminal_size = size().unwrap();
            let term_height = terminal_size.1 as usize;
            let mut i = 0;
            while i < term_height {
                *cursor_y = helper::down(&file_data, *cursor_y);
                i += 2;
            }
        } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
            let terminal_size = size().unwrap();
            let term_height = terminal_size.1 as usize;
            let mut i = 0;
            while i < term_height {
                *cursor_y = helper::up(*cursor_y);
                i += 2;
            }
        } else if *prev_keys == "g" && code == KeyCode::Char('c') {
            helper::log_command(code, modifiers, last_command, *recording);
            let comment_string = match helper::get_comment_string(file_name) {
                Some(chr) => chr,
                None => "#",
            };
            file_data[*cursor_y] = helper::toggle_comment(file_data[*cursor_y].clone(), comment_string);
            *cursor_x = helper::count_leading_spaces(&file_data[*cursor_y]);
            *prev_keys = "".to_string();
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if *prev_keys == "c" && code == KeyCode::Char('c') {
            helper::log_command(code, modifiers, last_command, *recording);
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *cursor_x, *cursor_y, 'V');
            helper::delete_in_visual_and_insert(file_data, *cursor_y, *cursor_y);
            *cursor_x = 0; // TODO make indent level
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'i';
            *prev_keys = "".to_string();
        } else if *prev_keys == "y" && code == KeyCode::Char('y') {
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *cursor_x, *cursor_y, 'V');
            *prev_keys = "".to_string();
        } else if *prev_keys == "d" && code == KeyCode::Char('d') {
            helper::log_command(code, modifiers, last_command, *recording);
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *cursor_x, *cursor_y, 'V');
            helper::delete_in_visual(file_data, *cursor_x, *cursor_y, *cursor_x, *cursor_y, 'V');
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *prev_keys = "".to_string();
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if *prev_keys == "" && code == KeyCode::Char('g') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "g".to_string();
        } else if *prev_keys == "" && code == KeyCode::Char('r') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "r".to_string();
        } else if *prev_keys == "" && code == KeyCode::Char('c') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "c".to_string();
        } else if *prev_keys == "" && code == KeyCode::Char('d') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "d".to_string();
        } else if *prev_keys == "" && code == KeyCode::Char('y') {
            last_command.clear();
            helper::log_command(code, modifiers, last_command, *recording);
            *prev_keys = "y".to_string();
        } else if code == KeyCode::Char('G') {
            *cursor_y = file_data.len() - 1;
        } else if code == KeyCode::Char('/') {
            *mode = '/';
            *search_string = "".to_string();
            *searching = true;
        } else if code == KeyCode::Char('N') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            (*cursor_x, *cursor_y) = match helper::get_prev_occurrence(file_data, *cursor_x, *cursor_y, search_string) {
                Some((x, y)) => (x, y),
                None => (*cursor_x, *cursor_y)
            };
            *searching = true;
        } else if code == KeyCode::Char('n') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            (*cursor_x, *cursor_y) = match helper::find_next_occurrence(file_data, *cursor_x + 1, *cursor_y, search_string) {
                Some((x, y)) => (x, y),
                None => (*cursor_x, *cursor_y)
            };
            *searching = true;
        } else if code == KeyCode::Esc {
            *prev_keys = "".to_string();
        }
    } else if *mode == '/' {
        if code == KeyCode::Esc {
            *mode = 'n';
        } else if code == KeyCode::Enter {
            *mode = 'n';
        } else if code == KeyCode::Backspace {
            search_string.pop();
        } else if let KeyCode::Char(c) = code {
            search_string.push(c);
        }
        (*cursor_x, *cursor_y) = match helper::find_next_occurrence(file_data, *cursor_x, *cursor_y, search_string) {
            Some((x, y)) => (x, y),
            None => (*cursor_x, *cursor_y)
        };
        helper::log_command(code, modifiers, last_command, *recording);
    } else if *mode == 'i' {
        if code == KeyCode::Esc {
            *mode = 'n';
            *cursor_x = helper::left(*cursor_x);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::BackTab {
            file_data[*cursor_y] = helper::reduce_indent(file_data[*cursor_y].clone());
            if *cursor_x >= 4 {
                *cursor_x -= 4;
            } else {
                *cursor_x = 0;
            }
        } else if code == KeyCode::Tab {
            file_data[*cursor_y] = helper::increase_indent(file_data[*cursor_y].clone());
            *cursor_x += 4;
        } else if code == KeyCode::Enter {
            let mut indent_level = helper::count_leading_spaces(&file_data[*cursor_y]);
            if file_data[*cursor_y][..*cursor_x].ends_with('(') || file_data[*cursor_y][..*cursor_x].ends_with('{') {
                indent_level += 4;
            }
            let substring = " ".repeat(indent_level) + &file_data[*cursor_y][*cursor_x..];
            if file_data[*cursor_y][..*cursor_x].ends_with('(') {
                file_data.insert(*cursor_y + 1, " ".repeat(indent_level - 4) + ")");
            }
            if file_data[*cursor_y][..*cursor_x].ends_with('{') {
                file_data.insert(*cursor_y + 1, " ".repeat(indent_level - 4) + "}");
            }
            file_data.insert(*cursor_y + 1, substring.to_string());
            file_data[*cursor_y] = file_data[*cursor_y][..*cursor_x].to_string();
            *cursor_y += 1;
            *cursor_x = indent_level;
        } else if code == KeyCode::Backspace {
            if *cursor_x > 0 {
                file_data[*cursor_y].remove(*cursor_x - 1);
                *cursor_x = helper::left(*cursor_x);
            }
        } else if code == KeyCode::Delete {
            file_data[*cursor_y].remove(*cursor_x);
        } else if let KeyCode::Char(c) = code {
            file_data[*cursor_y].insert(*cursor_x, c);
            *cursor_x += 1;
        }
        helper::log_command(code, modifiers, last_command, *recording);
    } else if *mode == 'v' {
        if code == KeyCode::Esc {
            *mode = 'n';
        } else if code == KeyCode::Char('$') {
            *cursor_x = helper::set_cursor_end(&file_data, *cursor_y);
            *cursor_x = helper::left(*cursor_x);
        } else if code == KeyCode::Char('^') {
            *cursor_x = helper::count_leading_spaces(&file_data[*cursor_y]);
        } else if code == KeyCode::Char('0') {
            *cursor_x = 0;
        } else if code == KeyCode::Char('*') {
            *mode = 'n';
            *searching = true;
            (*cursor_x, *visual_x) = helper::normalize(*cursor_x, *visual_x);
            *search_string = file_data[*cursor_y][*cursor_x..*visual_x+1].to_string();
        } else if code == KeyCode::Char('{') {
            *cursor_y = helper::get_prev_empty_line(&file_data, *cursor_y);
        } else if code == KeyCode::Char('}') {
            *cursor_y = helper::get_next_empty_line(&file_data, *cursor_y);
        } else if code == KeyCode::Char('h') {
            *cursor_x = helper::left(*cursor_x);
        } else if code == KeyCode::Char('l') {
            *cursor_x = helper::right(&file_data, *cursor_x, *cursor_y);
        } else if code == KeyCode::Char('j') {
            *cursor_y = helper::down(&file_data, *cursor_y);
        } else if code == KeyCode::Char('k') {
            *cursor_y = helper::up(*cursor_y);
        } else if code == KeyCode::Char('b') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *cursor_x = helper::get_index_prev_word(&file_data, *cursor_x, *cursor_y);
        } else if code == KeyCode::Char('w') {
            *cursor_x = helper::reset_cursor_end(&file_data, *cursor_x, *cursor_y);
            *cursor_x = helper::get_index_next_word(&file_data, *cursor_x, *cursor_y);
        } else if *prev_keys == "g" && code == KeyCode::Char('g') {
            *cursor_y = 0;
            *prev_keys = "".to_string();
        } else if *prev_keys == "g" && code == KeyCode::Char('c') {
            let comment_string = match helper::get_comment_string(file_name) {
                Some(chr) => chr,
                None => "#",
            };
            helper::toggle_comments_in_visual(file_data, comment_string, *cursor_y, *visual_y);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *prev_keys = "".to_string();
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if *prev_keys == "" && code == KeyCode::Char('g') {
            *prev_keys = "g".to_string();
        } else if code == KeyCode::Char('G') {
            *cursor_y = file_data.len() - 1;
        } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
            let terminal_size = size().unwrap();
            let term_height = terminal_size.1 as usize;
            let mut i = 0;
            while i < term_height {
                *cursor_y = helper::down(&file_data, *cursor_y);
                i += 2;
            }
        } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
            let terminal_size = size().unwrap();
            let term_height = terminal_size.1 as usize;
            let mut i = 0;
            while i < term_height {
                *cursor_y = helper::up(*cursor_y);
                i += 2;
            }
        } else if code == KeyCode::Char('y') {
            *cursor_x = helper::prevent_cursor_end(&file_data, *cursor_x, *cursor_y);
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_x = helper::get_cursor_after_visual(*cursor_x, *visual_x);
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('c') {
            *cursor_x = helper::prevent_cursor_end(&file_data, *cursor_x, *cursor_y);
            (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'i';
        } else if code == KeyCode::Char('d') {
            *cursor_x = helper::prevent_cursor_end(&file_data, *cursor_x, *cursor_y);
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('x') {
            (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        }
        helper::log_command(code, modifiers, last_command, *recording);
    } else if *mode == 'V' {
        if code == KeyCode::Esc {
            *mode = 'n';
        } else if code == KeyCode::Char('{') {
            *cursor_y = helper::get_prev_empty_line(&file_data, *cursor_y);
        } else if code == KeyCode::Char('}') {
            *cursor_y = helper::get_next_empty_line(&file_data, *cursor_y);
        } else if code == KeyCode::Char('j') {
            *cursor_y = helper::down(&file_data, *cursor_y);
        } else if code == KeyCode::Char('k') {
            *cursor_y = helper::up(*cursor_y);
        } else if *prev_keys == "g" && code == KeyCode::Char('g') {
            *cursor_y = 0;
            *prev_keys = "".to_string();
        } else if *prev_keys == "g" && code == KeyCode::Char('c') {
            let comment_string = match helper::get_comment_string(file_name) {
                Some(chr) => chr,
                None => "#",
            };
            helper::toggle_comments_in_visual(file_data, comment_string, *cursor_y, *visual_y);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *prev_keys = "".to_string();
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if *prev_keys == "" && code == KeyCode::Char('g') {
            *prev_keys = "g".to_string();
        } else if code == KeyCode::Char('G') {
            *cursor_y = file_data.len() - 1;
        } else if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
            let terminal_size = size().unwrap();
            let term_height = terminal_size.1 as usize;
            let mut i = 0;
            while i < term_height {
                *cursor_y = helper::down(&file_data, *cursor_y);
                i += 2;
            }
        } else if code == KeyCode::Char('u') && modifiers.contains(KeyModifiers::CONTROL) {
            let terminal_size = size().unwrap();
            let term_height = terminal_size.1 as usize;
            let mut i = 0;
            while i < term_height {
                *cursor_y = helper::up(*cursor_y);
                i += 2;
            }
        } else if code == KeyCode::Char('>') {
            helper::increase_indent_visual(file_data, *cursor_y, *visual_y);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *mode = 'n';
        } else if code == KeyCode::Char('<') {
            helper::reduce_indent_visual(file_data, *cursor_y, *visual_y);
            helper::save_to_file(file_data, file_name, diff_history, *pos);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *mode = 'n';
        } else if code == KeyCode::Char('y') {
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('c') {
            helper::delete_in_visual_and_insert(file_data, *cursor_y, *visual_y);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'i';
        } else if code == KeyCode::Char('d') {
            helper::copy_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        } else if code == KeyCode::Char('x') {
            (*cursor_x, *cursor_y) = helper::delete_in_visual(file_data, *cursor_x, *cursor_y, *visual_x, *visual_y, *mode);
            *cursor_y = helper::get_cursor_after_visual(*cursor_y, *visual_y);
            *cursor_y = helper::reset_cursor_end_file(file_data.len(), *cursor_y);
            *mode = 'n';
            helper::save_to_file(file_data, file_name, diff_history, *pos);
        }
        helper::log_command(code, modifiers, last_command, *recording);
    }
}

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
    let mut prev_keys = "".to_string();
    let mut last_command: Vec<(KeyCode, KeyModifiers)> = Vec::new();
    let mut recording = true;
    let mut searching = false;
    let mut search_string = "".to_string();
    let mut prev_view: Vec<Vec<(char, Color, Color, bool)>> = Vec::new();
    let mut macro_command: Vec<(KeyCode, KeyModifiers)> = Vec::new();
    let mut macro_recording = false;
    let mut diff_history = diffhist::DiffHistory::new(file_data.clone());
    let mut pos = (0, 0);
    if file_data.len() == 0 {
        file_data.insert(0, "".to_string());
    }
    prev_view = helper::render_file_data(
        prev_view.clone(),
        file_name,
        &file_data,
        window_line_x,
        window_line_y,
        cursor_x,
        cursor_y,
        visual_x,
        visual_y,
        mode,
        search_string.clone(),
        searching,
        macro_recording,
        false,
    );
    loop {
        if let Ok(event) = crossterm::event::read() {
            let mut key_code: Option<KeyCode> = None;
            let mut key_modifiers: Option<KeyModifiers> = None;
            let mut resize = false;
            match event {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    key_code = Some(code);
                    key_modifiers = Some(modifiers);
                    if macro_recording && !(mode == 'n' && code == KeyCode::Char('q')) {
                        macro_command.push((code, modifiers));
                    }
                },
                Event::Resize(_, _) => {
                    resize = true;
                },
                _ => break,
            }
            if key_code != None && key_code.unwrap() == KeyCode::Char('c') && key_modifiers.unwrap().contains(KeyModifiers::CONTROL) {
                break;
            } else {
                if !resize {
                    send_command(
                        key_code.unwrap(),
                        key_modifiers.unwrap(),
                        &mut file_data,
                        file_name,
                        &mut cursor_x,
                        &mut cursor_y,
                        &mut visual_x,
                        &mut visual_y,
                        &mut mode,
                        &mut prev_keys,
                        &mut last_command,
                        &mut recording,
                        &mut search_string,
                        &mut searching,
                        &mut macro_command,
                        &mut macro_recording,
                        &mut diff_history,
                        &mut pos,
                    );
                }
                (window_line_x, window_line_y) = helper::calc_window_lines(&file_data, window_line_x, window_line_y, cursor_x, cursor_y);
                prev_view = helper::render_file_data(
                    prev_view.clone(),
                    file_name,
                    &file_data,
                    window_line_x,
                    window_line_y,
                    cursor_x,
                    cursor_y,
                    visual_x,
                    visual_y,
                    mode,
                    search_string.clone(),
                    searching,
                    macro_recording,
                    resize,
                );
            }
        }
    }
    helper::quit_terminal();
}
