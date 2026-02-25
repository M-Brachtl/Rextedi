pub mod tui {
    use std::char;
    use std::io::{stdout, Write};
    use crossterm::terminal::{Clear, ClearType};
    use crossterm::{self, execute, queue};
    use crossterm::style::{Color, Print, SetForegroundColor, Stylize, SetBackgroundColor};
    use crossterm::cursor::{MoveTo, Hide, Show};
    use unicode_segmentation::UnicodeSegmentation;

    use crate::move_text;

    static HEADER_HEIGHT: u16 = 3;

    fn grapheme_length(line: &str, len: usize) -> usize {
        line.grapheme_indices(true).nth(len).map(|(i, _)| i).unwrap_or(line.graphemes(true).count())
    }
    fn grapheme_getindex(line: &str, index: usize) -> usize {
        line.grapheme_indices(true).nth(index).map(|(i, _)| i).unwrap_or(line.len()) // at the end has to be len to get byte index, not grapheme index
    }

    pub struct Editor {
        cursor: (usize, usize), // (column, line) / (x, y)
        width: usize,
        height: usize, // height of the editor area (excluding header and log line)
        content: Vec<String>, // content of the editor, each string is a line
        local_hor_offset: usize, // local horizontal offset for the current line (for handling long lines)
        _hor_offset: usize, // horizontal offset for scrolling
        ver_offset: usize, // vertical offset for scrolling
    }
    impl Editor {
        pub fn new(width: usize, height: usize) -> Self {
            Self {
                cursor: (0, 0),
                width,
                height: height - HEADER_HEIGHT as usize - 1, // -1 for log line
                content: vec![String::new()], // Start with one empty line
                local_hor_offset: 0, // local horizontal offset for the current line (for handling long lines)
                _hor_offset: 0, // horizontal offset for scrolling
                ver_offset: 0, // vertical offset for scrolling
            }
        }

        pub fn write_char(&mut self, char: char) {
            let line = &mut self.content[self.cursor.1];
            let byte_index = grapheme_getindex(line, self.cursor.0 + self.local_hor_offset);
            line.insert(byte_index, char);
            self.cursor.0 += 1;
            self.update_line(self.cursor.1).unwrap();
        }

        pub fn delete_char(&mut self) {
            if self.cursor.0 == 0 && self.cursor.1 == 0 {
                return; // Nothing to delete
            }
            if self.cursor.0 == 0 {
                // Merge with previous line
                let prev_line_len = self.content[self.cursor.1 - 1].graphemes(true).count();
                let current_line = self.content.remove(self.cursor.1);
                self.content[self.cursor.1 - 1].push_str(&current_line);
                self.cursor.0 = prev_line_len;
                self.cursor.1 -= 1;
            } else {
                let line = &mut self.content[self.cursor.1];
                let byte_index = grapheme_getindex(line, self.cursor.0 - 1);
                line.remove(byte_index);
                self.cursor.0 -= 1;
            }
            self.update_line(self.cursor.1).unwrap();
        }

        pub fn insert_newline(&mut self) {
            let line = &mut self.content[self.cursor.1];
            let byte_index = grapheme_getindex(line, self.cursor.0);
            let new_line = line.split_off(byte_index);
            self.content.insert(self.cursor.1 + 1, new_line);
            self.local_hor_offset = 0; // reset local horizontal offset when moving vertically
            // self.update_line(self.cursor.1).unwrap();
            self.update_cursor_down().unwrap();
            self.cursor.1 += 1;
            self.cursor.0 = 0;
        }

        fn move_visible(&self) {
        }

        pub fn update_line(&self, index: usize) -> Result<(), std::io::Error> { // draws lines from index_range.0 to index_range.1 (inclusive)
            let offset_bytes = grapheme_getindex(&self.content[index], self.local_hor_offset);
            let display_line = self.content[index][offset_bytes..].graphemes(true).take(self.width).collect::<String>();
            queue!(
                stdout(),
                crossterm::cursor::Hide,
                MoveTo(0, index as u16 + HEADER_HEIGHT),
                Clear(ClearType::CurrentLine),
                Print(display_line),
                MoveTo(self.cursor.0 as u16, self.cursor.1 as u16 + HEADER_HEIGHT),
                crossterm::cursor::Show
            )?;
            stdout().flush()
        }
        pub fn update_cursor_down(&mut self) -> Result<(), std::io::Error> {
            let (_, moved_lines) = self.content.split_at((self.cursor.1 + self.ver_offset) as usize);
            queue!(stdout(), crossterm::cursor::Hide, Clear(ClearType::FromCursorDown))?;
            if self.ver_offset > moved_lines.len() {
                queue!(
                    stdout(),
                    MoveTo(0, self.cursor.1 as u16 + HEADER_HEIGHT),
                    // Clear(ClearType::CurrentLine),
                )?;
                stdout().flush()?;
                return Ok(()); // No need to redraw if the new line is outside the current view
            }
            for (index, line) in moved_lines[self.ver_offset as usize..].iter().take(self.height - (self.cursor.1 + self.ver_offset) as usize).enumerate(){
                queue!(
                    stdout(),
                    MoveTo(0, self.cursor.1 as u16 + index as u16 + HEADER_HEIGHT),
                    Clear(ClearType::CurrentLine),
                    Print(if line.graphemes(true).count() > self.width {
                        line.split_at(self.width).0
                    } else {
                        line
                    }),
                )?;
            }
            queue!(
                stdout(),
                MoveTo(self.cursor.0 as u16, self.cursor.1 as u16 + HEADER_HEIGHT),
                crossterm::cursor::Show
            )?;
            stdout().flush()
        }
        pub fn move_cursor(&mut self, direction: super::Direction){
            if matches!(direction, super::Direction::Up | super::Direction::Down) {
                self.local_hor_offset = 0;
                self.update_line(self.cursor.1).unwrap();
            }
            let orig_ver_offset = self.ver_offset;
            match direction {
                super::Direction::Up => self.cursor.1 -= if self.cursor.1 > 0 { 1 } else if self.ver_offset > 0 { self.ver_offset -= 1; 1 } else { 0 },
                super::Direction::Down => self.cursor.1 += if self.cursor.1 + 1 < self.height && self.ver_offset + 1 < self.content.len() { 1 } else if self.ver_offset + 1 < self.content.len() { self.ver_offset += 1; 1 } else { 0 },
                super::Direction::Left => self.cursor.0 -= if self.cursor.0 > 0 { 1 } else { 0 },
                super::Direction::Right => {
                    if self.cursor.0 + self.local_hor_offset < self.content[self.cursor.1].graphemes(true).count() {
                        self.cursor.0 += 1;
                    }
                }
            }
            if self.cursor.0 + self.local_hor_offset >= self.content[self.cursor.1 + self.ver_offset].graphemes(true).count() {
                self.cursor.0 = self.content[self.cursor.1 + self.ver_offset].graphemes(true).count() - self.local_hor_offset;
            }
            execute!(stdout(), MoveTo(self.cursor.0 as u16, self.cursor.1 as u16 + HEADER_HEIGHT)).unwrap();
            if orig_ver_offset != self.ver_offset {
                self.move_visible();
            } else {
                self.update_line(self.cursor.1).unwrap();
            }
        }
    }

    pub struct Window {
        pub(crate) width: usize, // total width of the terminal
        pub(crate) height: usize,
    }
    impl Window {
        pub fn new() -> Self {
            crossterm::terminal::enable_raw_mode().unwrap();
            let size = crossterm::terminal::size().unwrap();
            let window = Self {
                width: size.0 as usize,
                height: size.1 as usize,
            };
            execute!(stdout(), Clear(ClearType::All), crossterm::terminal::EnterAlternateScreen, Hide).unwrap();
            Self::draw_header(&window).unwrap();
            window
        }

        fn draw_header(&self) -> Result<(), std::io::Error> {
            let header = " Rextedi \r\n".bold().with(Color::DarkMagenta).on(Color::White);
            let subheader = "^Q to quit\t ^W to write into a file\t ^[→] to move line right\t ^[←] to move line left\r\n".with(Color::Blue);
            execute!(
                stdout(),
                MoveTo(0, 0),
                Print(header), 
                Print(subheader),
                Print("─".repeat(self.width)), // Horizontal line after header
                MoveTo(0, HEADER_HEIGHT),
            )?;
            stdout().flush()?;
            Ok(())
        }
    }
}
pub enum Direction { Up, Down, Left, Right }