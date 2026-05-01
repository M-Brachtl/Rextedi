mod render;

use std::{env, fs, io::{Write, stdout}, path::{Path, PathBuf}, vec};

use crossterm::{
    cursor::{MoveTo, Show},
    event::{Event, KeyCode, KeyModifiers, read, MouseEventKind},
    execute, terminal::{
        LeaveAlternateScreen, disable_raw_mode
    }
};

use unicode_segmentation::UnicodeSegmentation;
use render::tui;
use render::Direction;

use crate::render::tui::Log;

static DEBUG_MODE: bool = false; // Set to true to enable debug log at the bottom of the screen (overwrites normal log)

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();
    let mut run_args = env::args();
    let _ = run_args.next(); // skip executed file arg
    let mut file_content = String::new();
    let filepath = if let Some(ref filename) = run_args.next() {
        file_content = load_file(Path::new(filename));
        PathBuf::from(filename.clone())
    } else {
        PathBuf::new()
    };
    if filepath == PathBuf::new() {
        println!("Rextedi: No file path");
        return Ok(());
    }
    // Enable raw mode
    //enable_raw_mode()?;
    //execute!(stdout, Clear(ClearType::All), EnterAlternateScreen, Hide)?;
    let window = tui::Window::new();
    let editor = tui::Editor::new(window.width, window.height, vec![String::new()]);

    let result = run(editor, window, file_content, filepath.as_path());

    // ALWAYS restore terminal
    disable_raw_mode()?;
    execute!(stdout, Show, LeaveAlternateScreen)?;

    result
}

fn load_file(file_name: &Path) -> String {
    match fs::read_to_string(file_name){
        Ok(content) => content,
        Err(_) => {
            String::new()
        },
    }
}

fn save_file(file_name: &Path, content: String, log: &mut Log, cursor: (usize, usize)) -> std::io::Result<()> {
    let result = fs::write(file_name, content);
    match result {
        Ok(_) => {
            log.contents.1 = "File saved".to_string();
            log.add_info_lifetime = 5; // Show "File saved" for 5 update cycles
            log.update(cursor)?;
            Ok(())
        },
        Err(e) => {
            eprintln!("Failed to write to file: {}", e);
            Err(e)
        }
    }
}

fn run(mut editor: tui::Editor, window: tui::Window, starting_text: String, file_name: &Path) -> std::io::Result<()> {
    // let mut input = String::new();
    let crlf = starting_text.contains("\r\n"); // If no end-of-line is found or new file, default to LF
    let lines = if starting_text != "" {
        starting_text.replace("\r\n", "\n").split('\n').map(|text| text.to_string()).collect()
    } else {
        vec!["".to_string()]
    };
    editor.set_content(lines);
    let mut log: tui::Log<'_> = tui::Log::new(&window);

    // let mut cursor: (usize, usize) = (0, 0); // Default cursor position
    // let size = crossterm::terminal::size().unwrap();
    // let (width, height) = (size.0 as usize, size.1 as usize - HEADER_HEIGHT as usize - 1); // -1 for log line
    // let mut hor_offset: usize = 0;
    // let mut ver_offset: usize = 0;
    let stdout = &mut stdout();
    loop {
        if DEBUG_MODE {
            let (active_line, cursor_x, cursor_y, hor_offset, ver_offset, cur_char_repr) = editor.read_state();
            log.contents.0 = format!("ActiveLine: {}, Cursor: ({}, {}), HorOffset: {}, VerOffset: {}, LineLen: {}, CurCharRepr: {}", active_line, cursor_x, cursor_y, hor_offset, ver_offset, editor.content[active_line].graphemes(true).count(), cur_char_repr);
            log.update(editor.cursor)?;
            // (stdout, &format!("Debug log: ActiveLine: {}, Cursor: ({}, {}), HorOffset: {}, VerOffset: {}, LineLen: {}, CurCharRepr: {}", active_line, cursor.0, cursor.1, hor_offset, ver_offset, lines[active_line].graphemes(true).count(), lines[active_line].graphemes(true).nth((cursor.0 + hor_offset) as usize).unwrap_or("\0").escape_debug()), height, cursor)?;
        } else {
            let (_active_line, cursor_x, cursor_y, hor_offset, ver_offset, _) = editor.read_state();
            log.contents.0 = format!("Ln {}, Col {}", cursor_y + ver_offset + 1, cursor_x + hor_offset + 1);
            log.update(editor.cursor)?;
            // + 1 for user-friendly 1-based indexing in log display
        }
        // if let Some(log_instance) = &log {
        //     if log_instance.timestamp.elapsed().unwrap().as_secs() < 5 {
        //         log_instance.draw_log(stdout, &log_instance.message, height, cursor)?;
        //     } else {
        //         log_instance.clear_log(stdout, height, cursor)?;
        //         log = None;
        //     }
        // }
        // Block until key press (no infinite printing)
        match read()? {
        // if let Event::Key(key) = read()? {
            Event::Key(key) => {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                                execute!(stdout, MoveTo(0, 0), LeaveAlternateScreen)?; break;
                            } else {
                                editor.write_char('q');
                            }
                        },
                        KeyCode::Char('w') => if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            if let Err(_) = save_file(file_name, editor.content.join(if crlf {"\r\n"} else {"\n"}), &mut log, editor.cursor) {
                                let f = fs::File::create(file_name);
                                match f {
                                    Ok(mut file) => {
                                        match file.write_all(editor.content.join(if crlf {"\r\n"} else {"\n"}).as_bytes()) {
                                            Ok(_) => {
                                                log.contents.1 = "File saved".to_string();
                                                log.add_info_lifetime = 5; // Show "File saved" for 5 update cycles
                                                log.update(editor.cursor)?;
                                            },
                                            Err(e) => {
                                                eprintln!("Failed to write to file: {}", e);
                                            }

                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("Failed to create file: {}", e);
                                    }
                                }
                            }
                        } else {
                            editor.write_char('w');
                        }
                        KeyCode::Char(c) => if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            editor.write_char(c);
                        }
                        KeyCode::Backspace => {
                            editor.delete_char();
                        }
                        KeyCode::Enter => {
                            editor.insert_newline();
                        },
                        KeyCode::Left => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                editor.hor_scroll(Direction::Left);
                            } else /*if cursor.0 > 0*/ {
                                editor.move_cursor(Direction::Left);
                            }
                        },
                        KeyCode::Right => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                editor.hor_scroll(Direction::Right);
                            } else {
                                editor.move_cursor(Direction::Right);
                            }
                        },
                        KeyCode::Up => {
                            editor.move_cursor(Direction::Up);
                        },
                        KeyCode::Down => {
                            editor.move_cursor(Direction::Down);
                        },
                        _ => {}
                    }
                }
            },
            Event::Mouse(m) => {
                match m.kind {
                    MouseEventKind::Down(_) => {
                        // code
                    },
                    MouseEventKind::Up(_) => {
                        // code
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    Ok(())
}

// fn init_draw(stdout: &mut std::io::Stdout, lines: &Vec<String>, max_width: usize, max_height: usize) -> std::io::Result<()> {
//     let mut display_lines = Vec::new();
//     for line in lines.iter().take(max_height) {
//         if line.graphemes(true).count() > max_width {
//             display_lines.push(line.split_at(max_width).0.to_string());
//         } else {
//             display_lines.push(line.clone());
//         }
//     }
//     //assert!(display_lines.len() > max_height, "This should not be happenig. DLines: {}, MaxHeight: {}", display_lines.len(), max_height); // now should panic always
//     let header = " Rextedi \r\n".bold().with(Color::DarkMagenta).on(Color::White);
//     let subheader = "^Q to quit\t ^W to write into a file\t ^[→] to move line right\t ^[←] to move line left\r\n".with(Color::Blue);
//     execute!(
//         stdout,
//         crossterm::cursor::Show,
//         MoveTo(0,0),
//         Clear(ClearType::All),
//         Print(header),
//         Print(subheader),
//         Print("─".repeat(max_width)), // Horizontal line after header
//         Print(display_lines.join("\r\n")),
//         MoveTo(0,HEADER_HEIGHT),
//     )?;
//     stdout.flush()?;
//     Ok(())
// }

// fn draw(stdout: &mut std::io::Stdout, line: &String, cursor_pos: (usize, usize), max_width: usize, offset: usize) -> std::io::Result<()> {
//     let offset_bytes = line.grapheme_indices(true).nth(offset).map(|(i, _)| i).unwrap_or(line.graphemes(true).count());
//     let display_line = line[offset_bytes..].graphemes(true).take(max_width).collect::<String>();
//     queue!(
//         stdout,
//         crossterm::cursor::Hide,
//         MoveTo(0, cursor_pos.1 as u16 + HEADER_HEIGHT),
//         Clear(ClearType::CurrentLine),
//         Print(display_line),
//         MoveTo(cursor_pos.0 as u16, cursor_pos.1 as u16 + HEADER_HEIGHT),
//         crossterm::cursor::Show
//     )?;
//     stdout.flush()?;
//     Ok(())
// }

// fn draw_new_line(stdout: &mut std::io::Stdout, lines: &Vec<String>, cursor_pos: (usize, usize), max_width: usize, max_height: usize, ver_offset: usize) -> std::io::Result<()> {
//     let (_, moved_lines) = lines.split_at((cursor_pos.1 + ver_offset) as usize);
//     queue!(stdout, crossterm::cursor::Hide, Clear(ClearType::FromCursorDown))?;
//     if ver_offset > moved_lines.len() {
//         queue!(
//             stdout,
//             MoveTo(0, cursor_pos.1 as u16 + HEADER_HEIGHT),
//             // Clear(ClearType::CurrentLine),
//         )?;
//         stdout.flush()?;
//         return Ok(()); // No need to redraw if the new line is outside the current view
//     }
//     for (index, line) in moved_lines[ver_offset as usize..].iter().take(max_height - (cursor_pos.1 + ver_offset) as usize).enumerate(){
//         queue!(
//             stdout,
//             MoveTo(0, cursor_pos.1 as u16 + index as u16 + HEADER_HEIGHT),
//             Clear(ClearType::CurrentLine),
//             Print(if line.graphemes(true).count() > max_width {
//                 line.split_at(max_width).0
//             } else {
//                 line
//             }),
//         )?;
//     }
//     queue!(
//         stdout,
//         MoveTo(cursor_pos.0 as u16, cursor_pos.1 as u16 + HEADER_HEIGHT),
//         crossterm::cursor::Show
//     )?;
//     stdout.flush()?;
//     Ok(())
// }

// fn write_char(line: &mut String, c: char, cursor_x: &mut usize, hor_offset: &usize) {
//     let byte_index = line.grapheme_indices(true).nth((*cursor_x + *hor_offset) as usize).map(|(i, _)| i).unwrap_or(line.len()); // at the end has to be len to get byte index, not grapheme index
//     line.insert(byte_index, c);
//     *cursor_x += 1;
// }

// fn remove_char(line: &mut String, cursor_x: &mut usize, hor_offset: &usize) {
//     let byte_index = line.grapheme_indices(true).nth((*cursor_x + *hor_offset) as usize - 1).map(|(i, _)| i).unwrap_or(line.len()); // at the end has to be len to get byte index, not grapheme index
//     line.remove(byte_index);
//     *cursor_x -= 1;
// }

// fn move_text(ver_offset: usize, lines: &Vec<String>, cursor_pos: (usize, usize), stdout: &mut std::io::Stdout, width: usize, height: usize) -> std::io::Result<()> {
//     let (_, display_lines) = lines.split_at(ver_offset as usize);
//     queue!(stdout, crossterm::cursor::Hide, Clear(ClearType::FromCursorDown))?;
//     for (index, line) in display_lines.iter().take(height).enumerate(){
//         queue!(
//             stdout,
//             MoveTo(0, index as u16 + HEADER_HEIGHT),
//             Clear(ClearType::CurrentLine),
//             Print(if line.graphemes(true).count() > width {
//                 line.split_at(width).0
//             } else {
//                 line
//             }),
//         )?;
//     }
//     queue!(
//         stdout,
//         MoveTo(cursor_pos.0 as u16, cursor_pos.1 as u16 + HEADER_HEIGHT),
//         crossterm::cursor::Show
//     )?;
//     stdout.flush()?;
//     Ok(())
// }

/*struct Log {
    message: String,
    timestamp: std::time::SystemTime,
}

impl Log {
    fn new(message: String) -> Self {
        Self {
            message,
            timestamp: std::time::SystemTime::now(),
        }
    }
    fn draw_log(&self, stdout: &mut std::io::Stdout, message: &str, height: usize, cursor_pos: (usize, usize)) -> std::io::Result<()> {
        // assert!(false, "This means the func is being called. #YAY");
        execute!(
            stdout,
            MoveTo(0, height as u16 + HEADER_HEIGHT),
            Clear(ClearType::CurrentLine),
            Print(message),
            MoveTo(cursor_pos.0 as u16, cursor_pos.1 as u16 + HEADER_HEIGHT),
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn clear_log(&self, stdout: &mut std::io::Stdout, height: usize, cursor_pos: (usize, usize)) -> std::io::Result<()> {
        execute!(
            stdout,
            MoveTo(0, height as u16 + HEADER_HEIGHT),
            Clear(ClearType::CurrentLine),
            MoveTo(cursor_pos.0 as u16, cursor_pos.1 as u16 + HEADER_HEIGHT),
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn draw_debug(&self, stdout: &mut std::io::Stdout, message: &str, height: usize, cursor_pos: (usize, usize)) -> std::io::Result<()> {
        execute!(
            stdout,
            MoveTo(0, height as u16 + HEADER_HEIGHT),
            Clear(ClearType::CurrentLine),
            Print(message.with(Color::DarkYellow)),
            MoveTo(cursor_pos.0 as u16, cursor_pos.1 as u16 + HEADER_HEIGHT),
        )?;
        stdout.flush()?;
        Ok(())
    }
}*/
