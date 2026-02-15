use std::{io::{Write, stdout}, vec};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{Event, KeyCode, read},
    execute,
    style::{Print, Stylize},
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode
    },
};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    // Enable raw mode
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let result = run(&mut stdout);

    // ALWAYS restore terminal
    disable_raw_mode()?;
    execute!(stdout, Show, LeaveAlternateScreen)?;

    result
}

fn run(stdout: &mut std::io::Stdout) -> std::io::Result<()> {
    // let mut input = String::new();
    let mut lines = vec!["".to_string()];
    let mut active_line = 0;
    let mut cursor = crossterm::cursor::position()?;

    // let height = crossterm::terminal::size()?.1;

    init_draw(stdout)?;
    loop {
        draw(stdout, &lines[active_line], cursor)?;
        // Block until key press (no infinite printing)
        if let Event::Key(key) = read()? {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) { break; } else {
                        // lines[active_line].push('q');
                        write_char(&mut lines[active_line], 'q', &mut cursor.0);
                    },
                    KeyCode::Char(c) => if !key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) { write_char(&mut lines[active_line], c, &mut cursor.0); }
                    KeyCode::Backspace => {
                        if cursor.0 > 0 {
                            // lines[active_line].remove(idx);
                            // cursor.0 -= 1;
                            remove_char(&mut lines[active_line], &mut cursor.0);
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + 3))?;
                        } else {
                            let removed_line_content = lines[active_line].clone();
                            lines[active_line-1].push_str(&removed_line_content);
                            lines.remove(active_line);
                            active_line -= 1;
                            draw_new_line(stdout, &lines, cursor)?;
                            cursor.0 = lines[active_line].len() as u16 - removed_line_content.len() as u16;
                            cursor.1 -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        let new_line_content = lines[active_line].split_off(cursor.0 as usize);
                        lines.insert(active_line + 1, new_line_content);
                        draw_new_line(stdout, &lines, cursor)?;
                        active_line += 1;
                        cursor.0 = 0;
                        cursor.1 += 1;
                    },
                    KeyCode::Left => {
                        if cursor.0 > 0 {
                            cursor.0 -= 1;
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + 3))?;
                        }
                    },
                    KeyCode::Right => {
                        if cursor.0 < lines[active_line].len() as u16 {
                            cursor.0 += 1;
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + 3))?;
                        }
                    },
                    KeyCode::Up => {
                        if active_line > 0 {
                            active_line -= 1;
                            cursor.1 -= 1;
                            if cursor.0 > lines[active_line].len() as u16 {
                                cursor.0 = lines[active_line].len() as u16;
                            }
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + 3))?;
                        }
                    },
                    KeyCode::Down => {
                        if active_line < lines.len() - 1 {
                            active_line += 1;
                            cursor.1 += 1;
                            if cursor.0 > lines[active_line].len() as u16 {
                                cursor.0 = lines[active_line].len() as u16;
                            }
                            execute!(stdout, MoveTo(cursor.0, cursor.1 + 3))?;
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn init_draw(stdout: &mut std::io::Stdout) -> std::io::Result<()> {
    execute!(
        stdout,
        crossterm::cursor::Show,
        MoveTo(0,0),
        Clear(ClearType::All),
        Print("Persistent Crossterm TUI\n".bold()),
        Print("Press 'ctrl + q' to quit\n\n"),
        Print("Your input:\n"),
    )?;
    stdout.flush()?;
    Ok(())
}

fn draw(stdout: &mut std::io::Stdout, line: &String, cursor_pos: (u16, u16)) -> std::io::Result<()> {
    execute!(
        stdout,
        crossterm::cursor::Hide,
        MoveTo(0, cursor_pos.1 + 3),
        Clear(ClearType::CurrentLine),
        Print(line),
        MoveTo(cursor_pos.0, cursor_pos.1 + 3),
        crossterm::cursor::Show
    )?;
    stdout.flush()?;
    Ok(())
}

fn draw_new_line(stdout: &mut std::io::Stdout, lines: &Vec<String>, cursor_pos: (u16, u16)) -> std::io::Result<()> {
    let (_, moved_lines) = lines.split_at(cursor_pos.1 as usize);
    execute!(stdout, crossterm::cursor::Hide, Clear(ClearType::FromCursorDown))?;
    for (index, line) in moved_lines.iter().enumerate(){
        execute!(
            stdout,
            MoveTo(0, cursor_pos.1 + index as u16 + 3),
            Print(line),
        )?;
    }
    execute!(
        stdout,
        MoveTo(cursor_pos.0, cursor_pos.1 + 3),
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
    let byte_index = line.char_indices().nth(*cursor_x as usize).map(|(i, _)| i).unwrap_or(line.len());
    line.remove(byte_index);
    *cursor_x -= 1;
}