use std::{env, fs, io::{Write, stdout}, path::{Path, PathBuf}, vec};

use crossterm::{
    cursor::{Hide, MoveTo, Show}, event::{Event, KeyCode, KeyModifiers, read}, execute, queue, style::{Color, Print, Stylize}, terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode
    }
};

static HEADER_HEIGHT: u16 = 3;

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
    enable_raw_mode()?;
    execute!(stdout, Clear(ClearType::All), EnterAlternateScreen, Hide)?;

    let result = run(&mut stdout, file_content, filepath.as_path());

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

fn save_file(file_name: &Path, content: String) -> std::io::Result<()> {
    fs::write(file_name, content)
}

fn run(stdout: &mut std::io::Stdout, starting_text: String, file_name: &Path) -> std::io::Result<()> {
    // let mut input = String::new();
    let crlf = starting_text.contains("\r\n"); // If no end-of-line is found or new file, default to LF
    let mut lines = if starting_text != "" {
        starting_text.replace("\r\n", "\n").split('\n').map(|text| text.to_string()).collect()
    } else {
        vec!["".to_string()]
    };
    let mut log: Option<Log> = None;
    let mut active_line = 0;
    let mut cursor = crossterm::cursor::position()?;
    let (width, _height) = crossterm::terminal::size().unwrap();
    let mut hor_offset: u16 = 0;

    init_draw(stdout, &lines, width as usize)?;
    loop {
        draw(stdout, &lines[active_line], cursor, width as usize, hor_offset)?;
        if let Some(log_instance) = &log {
            if log_instance.timestamp.elapsed().unwrap().as_secs() < 5 {
                log_instance.draw_log(stdout, &log_instance.message, _height, cursor)?;
            } else {
                log_instance.clear_log(stdout, _height, cursor)?;
                log = None;
            }
        }
        // Block until key press (no infinite printing)
        if let Event::Key(key) = read()? {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) { execute!(stdout, MoveTo(0, 0), LeaveAlternateScreen)?; break; } else {
                        // lines[active_line].push('q');
                        write_char(&mut lines[active_line], 'q', &mut cursor.0);
                    },
                    KeyCode::Char('w') => if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        if let Err(_) = save_file(file_name, lines.join(if crlf {"\r\n"} else {"\n"})) {
                            let f = fs::File::create(file_name);
                            match f {
                                Ok(mut file) => {
                                    match file.write_all(lines.join(if crlf {"\r\n"} else {"\n"}).as_bytes()) {
                                        Ok(_) => {
                                            log = Some(Log::new("File saved".to_string()));
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
                        write_char(&mut lines[active_line], 'w', &mut cursor.0);
                    }
                    KeyCode::Char(c) => if !key.modifiers.contains(KeyModifiers::CONTROL) { write_char(&mut lines[active_line], c, &mut cursor.0); }
                    KeyCode::Backspace => {
                        if cursor.0 > 0 {
                            remove_char(&mut lines[active_line], &mut cursor.0);
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + HEADER_HEIGHT))?;
                        } else if cursor.1 > 0{
                            let removed_line_content = lines[active_line].clone();
                            lines[active_line-1].push_str(&removed_line_content);
                            lines.remove(active_line);
                            active_line -= 1;
                            draw_new_line(stdout, &lines, cursor, width as usize)?;
                            cursor.0 = lines[active_line].len() as u16 - removed_line_content.len() as u16;
                            cursor.1 -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        let new_line_content = lines[active_line].split_off((cursor.0 + hor_offset) as usize);
                        lines.insert(active_line + 1, new_line_content);
                        draw_new_line(stdout, &lines, cursor, width as usize)?;
                        hor_offset = 0;
                        active_line += 1;
                        cursor.0 = 0;
                        cursor.1 += 1;
                    },
                    KeyCode::Left => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            if hor_offset > 0 {
                                hor_offset -= 1;
                            }
                        } else if cursor.0 > 0 {
                            cursor.0 -= 1;
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + HEADER_HEIGHT))?;
                        }
                    },
                    KeyCode::Right => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            hor_offset += 1;
                        } else if cursor.0 < lines[active_line].len() as u16 {
                            cursor.0 += 1;
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + HEADER_HEIGHT))?;
                        }
                    },
                    KeyCode::Up => {
                        if active_line > 0 {
                            hor_offset = 0;
                            draw(stdout, &lines[active_line], cursor, width as usize, hor_offset)?;
                            active_line -= 1;
                            cursor.1 -= 1;
                            if cursor.0 > lines[active_line].len() as u16 {
                                cursor.0 = lines[active_line].len() as u16;
                            }
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + HEADER_HEIGHT))?;
                        }
                    },
                    KeyCode::Down => {
                        if active_line < lines.len() - 1 {
                            hor_offset = 0;
                            draw(stdout, &lines[active_line], cursor, width as usize, hor_offset)?;
                            active_line += 1;
                            cursor.1 += 1;
                            if cursor.0 > lines[active_line].len() as u16 {
                                cursor.0 = lines[active_line].len() as u16;
                            }
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + HEADER_HEIGHT))?;
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn init_draw(stdout: &mut std::io::Stdout, lines: &Vec<String>, max_width: usize) -> std::io::Result<()> {
    let mut display_lines = Vec::new();
    for line in lines {
        if line.len() > max_width {
            display_lines.push(line.split_at(max_width).0.to_string());
        } else {
            display_lines.push(line.clone());
        }
    }
    let header = " Rextedi \n".bold().with(Color::DarkMagenta).on(Color::White);
    let subheader = "^Q to quit\t ^W to write into a file\t ^[→] to move line right\t ^[←] to move line left\n".with(Color::Blue);
    execute!(
        stdout,
        crossterm::cursor::Show,
        MoveTo(0,0),
        Clear(ClearType::All),
        Print(header),
        Print(subheader),
        Print("─".repeat(max_width)), // Horizontal line after header
        Print(display_lines.join("\n")),
    )?;
    stdout.flush()?;
    Ok(())
}

fn draw(stdout: &mut std::io::Stdout, line: &String, cursor_pos: (u16, u16), max_width: usize, offset: u16) -> std::io::Result<()> {
    let offset_bytes = line.char_indices().nth(offset as usize).map(|(i, _)| i).unwrap_or(line.len());
    let display_line = line[offset_bytes..].chars().take(max_width).collect::<String>();
    execute!(
        stdout,
        crossterm::cursor::Hide,
        MoveTo(0, cursor_pos.1 + HEADER_HEIGHT),
        Clear(ClearType::CurrentLine),
        Print(display_line),
        MoveTo(cursor_pos.0, cursor_pos.1 + HEADER_HEIGHT),
        crossterm::cursor::Show
    )?;
    stdout.flush()?;
    Ok(())
}

fn draw_new_line(stdout: &mut std::io::Stdout, lines: &Vec<String>, cursor_pos: (u16, u16), max_width: usize) -> std::io::Result<()> {
    let (_, moved_lines) = lines.split_at(cursor_pos.1 as usize);
    queue!(stdout, crossterm::cursor::Hide, Clear(ClearType::FromCursorDown))?;
    for (index, line) in moved_lines.iter().enumerate(){
        queue!(
            stdout,
            MoveTo(0, cursor_pos.1 + index as u16 + HEADER_HEIGHT),
            Print(if line.len() > max_width {
                line.split_at(max_width).0
            } else {
                line
            }),
        )?;
    }
    queue!(
        stdout,
        MoveTo(cursor_pos.0, cursor_pos.1 + HEADER_HEIGHT),
        crossterm::cursor::Show
    )?;
    stdout.flush()?;
    Ok(())
}

fn write_char(line: &mut String, c: char, cursor_x: &mut u16) {
    let byte_index = line.char_indices().nth(*cursor_x as usize).map(|(i, _)| i).unwrap_or(line.len());
    line.insert(byte_index, c);
    *cursor_x += 1;
}

fn remove_char(line: &mut String, cursor_x: &mut u16) {
    let byte_index = line.char_indices().nth(*cursor_x as usize - 1).map(|(i, _)| i).unwrap_or(line.len());
    line.remove(byte_index);
    *cursor_x -= 1;
}

struct Log {
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
    fn draw_log(&self, stdout: &mut std::io::Stdout, message: &str, height: u16, cursor_pos: (u16, u16)) -> std::io::Result<()> {
        execute!(
            stdout,
            MoveTo(0, height - 1),
            Clear(ClearType::CurrentLine),
            Print(message),
            MoveTo(cursor_pos.0, cursor_pos.1 + HEADER_HEIGHT),
        )?;
        stdout.flush()?;
        Ok(())
    }
    
    fn clear_log(&self, stdout: &mut std::io::Stdout, height: u16, cursor_pos: (u16, u16)) -> std::io::Result<()> {
        execute!(
            stdout,
            MoveTo(0, height - 1),
            Clear(ClearType::CurrentLine),
            MoveTo(cursor_pos.0, cursor_pos.1 + HEADER_HEIGHT),
        )?;
        stdout.flush()?;
        Ok(())
    }
}