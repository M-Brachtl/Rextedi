pub mod tui {
    use std::char;
    use std::io::{stdout, Write};
    use crossterm::terminal::{Clear, ClearType};
    use crossterm::{self, execute, queue};
    use crossterm::style::{Color, Print, Stylize};
    use crossterm::cursor::{MoveTo};
    use unicode_segmentation::UnicodeSegmentation;

    static HEADER_HEIGHT: u16 = 3;

    fn grapheme_length(line: &str) -> usize {
        line.graphemes(true).count()
    }
    fn grapheme_getindex(line: &str, index: usize) -> usize {
        line.grapheme_indices(true).nth(index).map(|(i, _)| i).unwrap_or(line.len()) // at the end has to be len to get byte index, not grapheme index
    }

    pub struct Editor {
        pub(crate) cursor: (usize, usize), // (column, line) / (x, y)
        width: usize,
        height: usize, // height of the editor area (excluding header and log line)
        pub(crate) content: Vec<String>, // content of the editor, each string is a line
        local_hor_offset: usize, // local horizontal offset for the current line (for handling long lines)
        _hor_offset: usize, // horizontal offset for scrolling
        ver_offset: usize, // vertical offset for scrolling
    }
    impl Editor {
        pub fn new(width: usize, height: usize, content: Vec<String>) -> Self {
            Self {
                cursor: (0, 0),
                width,
                height: height - HEADER_HEIGHT as usize - 1, // -1 for log line
                content,
                local_hor_offset: 0, // local horizontal offset for the current line (for handling long lines)
                _hor_offset: 0, // horizontal offset for scrolling
                ver_offset: 0, // vertical offset for scrolling
            }
        }

        fn active_line_index(&self) -> usize {
            self.cursor.1 + self.ver_offset
        }

        pub fn read_state(&self) -> (usize, usize, usize, usize, usize, String) {
            let active_line = self.active_line_index();
            let hor_offset = self.local_hor_offset;
            let ver_offset = self.ver_offset;
            let cur_char_repr = self.content[active_line].graphemes(true).nth((self.cursor.0 + hor_offset) as usize).unwrap_or("\0").escape_debug().to_string();
            (active_line, self.cursor.0, self.cursor.1, hor_offset, ver_offset, cur_char_repr)
        }

        pub fn set_content(&mut self, content: Vec<String>) {
            self.content = content;
            for i in 0..self.content.len().min(self.height) {
                self.update_line(i).unwrap();
            }
        }

        pub fn write_char(&mut self, char: char) {
            let line = &mut self.content[self.cursor.1 + self.ver_offset];
            let byte_index = grapheme_getindex(line, self.cursor.0 + self.local_hor_offset);
            line.insert(byte_index, char);
            self.cursor.0 += 1;
            self.update_line(self.cursor.1).unwrap();
        }

        pub fn delete_char(&mut self) {
            if self.cursor.0 == 0 && self.cursor.1 + self.ver_offset == 0 {
                return; // Nothing to delete
            }
            if self.cursor.0 == 0 && self.cursor.1 == 0 {
                self.ver_offset -= 1;
                self.cursor.1 += 1;
                self.move_visible();
            }
            if self.cursor.0 == 0 {
                // Merge with previous line
                let prev_line_len = self.content[self.cursor.1 + self.ver_offset - 1].graphemes(true).count();
                let current_line = self.content.remove(self.cursor.1 + self.ver_offset);
                self.content[self.cursor.1 + self.ver_offset - 1].push_str(&current_line);
                self.cursor.0 = prev_line_len;
                self.local_hor_offset = 0; // reset local horizontal offset when moving vertically
                self.update_cursor_down().unwrap();
                self.cursor.1 -= 1;
            } else {
                let line = &mut self.content[self.cursor.1 + self.ver_offset];
                let byte_index = grapheme_getindex(line, self.cursor.0 + self.local_hor_offset - 1);
                line.remove(byte_index);
                self.cursor.0 -= 1;
            }
            self.update_line(self.cursor.1).unwrap();
        }

        pub fn insert_newline(&mut self) {
            let line = &mut self.content[self.cursor.1 + self.ver_offset];
            let byte_index = grapheme_getindex(line, self.cursor.0 + self.local_hor_offset);
            let new_line = line.split_off(byte_index);
            self.content.insert(self.cursor.1 + self.ver_offset + 1, new_line);
            self.local_hor_offset = 0; // reset local horizontal offset when moving vertically
            // self.update_line(self.cursor.1).unwrap();
            // self.update_cursor_down().unwrap();
            self.move_visible();
            self.cursor.1 += 1;
            self.cursor.0 = 0;
            if self.cursor.1 >= self.height {
                self.cursor.1 = self.height - 1;
                self.ver_offset += 1;
                self.move_visible();
            } else {
                execute!(stdout(), MoveTo(0, self.cursor.1 as u16 + HEADER_HEIGHT)).unwrap();
            }
        }

        fn move_visible(&self) {
            let mut stdout = stdout();
            queue!(
                stdout,
                crossterm::cursor::Hide,
                MoveTo(0, HEADER_HEIGHT),
                Clear(ClearType::FromCursorDown)
            ).unwrap();
            for (index, line) in self.content[self.ver_offset as usize..].iter().take(self.height).enumerate(){
                queue!(
                    stdout,
                    MoveTo(0, index as u16 + HEADER_HEIGHT),
                    Print(if line.graphemes(true).count() > self.width {
                        line.split_at(grapheme_getindex(line, self.local_hor_offset + self.width)).0
                    } else {
                        line
                    }),
                ).unwrap();
            }
            queue!(
                stdout,
                MoveTo(self.cursor.0 as u16, self.cursor.1 as u16 + HEADER_HEIGHT),
                crossterm::cursor::Show
            ).unwrap();
            stdout.flush().unwrap();
        }

        pub fn hor_scroll(&mut self, direction: super::Direction) {
            match direction {
                super::Direction::Left => {
                    if self.local_hor_offset > 0 {
                        self.local_hor_offset -= 1;
                    }
                },
                super::Direction::Right => {
                    self.local_hor_offset += 1;
                    if (self.local_hor_offset + self.cursor.0) >= grapheme_length(&self.content[self.cursor.1 + self.ver_offset]) {
                        self.cursor.0 = grapheme_length(&self.content[self.cursor.1 + self.ver_offset]) - self.local_hor_offset;
                    }
                },
                _ => {}
            }
            self.update_line(self.cursor.1).unwrap();
        }

        pub fn update_line(&self, index: usize) -> Result<(), std::io::Error> { // draws lines from index_range.0 to index_range.1 (inclusive)
            let offset_bytes = grapheme_getindex(&self.content[index + self.ver_offset], self.local_hor_offset);
            let display_line = self.content[index + self.ver_offset][offset_bytes..].graphemes(true).take(self.width).collect::<String>();
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
            let mut stdout = stdout();
            let cursor_down_content = &self.content[self.cursor.1 + self.ver_offset as usize..];
            let cdown_len = cursor_down_content.len();
            queue!(
                stdout,
                crossterm::cursor::Hide,
                MoveTo(0, self.cursor.1 as u16 + HEADER_HEIGHT),
                Clear(ClearType::FromCursorDown)
            )?;
            for (index, line) in cursor_down_content.iter().take(std::cmp::min(self.height - (self.cursor.1) as usize, cdown_len)).enumerate(){
                queue!(
                    stdout,
                    MoveTo(0, self.cursor.1 as u16 + index as u16 + HEADER_HEIGHT),
                    Print(if line.graphemes(true).count() > self.width {
                        line.split_at(grapheme_getindex(line, self.local_hor_offset + self.width)).0
                    } else {
                        line
                    }),
                )?;
            }
            queue!(
                stdout,
                MoveTo(self.cursor.0 as u16, self.cursor.1 as u16 + HEADER_HEIGHT),
                crossterm::cursor::Show
            )?;
            stdout.flush()
        }
        pub fn move_cursor(&mut self, direction: super::Direction){
            if matches!(direction, super::Direction::Up | super::Direction::Down) {
                self.local_hor_offset = 0;
                self.update_line(self.cursor.1).unwrap();
            }
            let orig_ver_offset = self.ver_offset;
            match direction {
                super::Direction::Up => self.cursor.1 -= if self.cursor.1 > 0 { 1 } else if self.ver_offset > 0 { self.ver_offset -= 1; self.move_visible(); 0 } else { 0 },
                super::Direction::Down => self.cursor.1 += if self.cursor.1 + 1 < self.height && self.cursor.1 + self.ver_offset + 1 < self.content.len() { 1 } else if self.ver_offset + self.cursor.1 + 1 < self.content.len() { self.ver_offset += 1; self.move_visible(); 0 } else { 0 },
                super::Direction::Left => self.cursor.0 -= if self.cursor.0 > 0 { 1 } else if self.local_hor_offset > 0 { self.local_hor_offset -= 1; 0 } else { 0 },
                super::Direction::Right => {
                    if self.cursor.0 + 1 < self.width && self.cursor.0 < grapheme_length(&self.content[self.cursor.1 + self.ver_offset]) {
                        self.cursor.0 += 1;
                    } else if self.cursor.0 + self.local_hor_offset < grapheme_length(&self.content[self.cursor.1 + self.ver_offset]) {
                        self.local_hor_offset += 1;
                    }
                }
            }
            if self.cursor.0 + self.local_hor_offset > grapheme_length(&self.content[self.cursor.1 + self.ver_offset]) {
                self.cursor.0 = grapheme_length(&self.content[self.cursor.1 + self.ver_offset]) - self.local_hor_offset;
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
            execute!(stdout(), Clear(ClearType::All), crossterm::terminal::EnterAlternateScreen, crossterm::cursor::Hide).unwrap();
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
    pub struct Log<'a> {
        pub(crate) contents: (String, String), // (main log, additional info, e.g. debug info, error details, etc.)
        pub(crate) add_info_lifetime: u8, // number of cycles, 0 means infinite lifetime
        window: &'a Window,
    }
    impl<'a> Log<'a> {
        pub fn new(window: &'a Window) -> Self {
            Self {
                contents: (String::new(), String::new()),
                add_info_lifetime: 0,
                window,
            }
        }
        pub fn update(&mut self, cursor: (usize, usize)) -> Result<(), std::io::Error> {
            // figure out lifetimes
            let mut stdout = stdout();
            queue!(
                stdout,
                crossterm::cursor::Hide,
                MoveTo(0, self.window.height as u16), // Position at the bottom of the editor area
                Clear(ClearType::CurrentLine),
                Print(("| ".to_string() + &self.contents.0 + " |").with(Color::DarkCyan)), // Main log content
                Print((if self.add_info_lifetime > 0 { " ".to_string() + &self.contents.1 } else { "".to_string() }).with(Color::Cyan)), // Additional info
                MoveTo(cursor.0 as u16, cursor.1 as u16 + HEADER_HEIGHT), // Move cursor back to its position
                crossterm::cursor::Show
            )?;
            self.add_info_lifetime = self.add_info_lifetime.saturating_sub(1); // Decrease lifetime of additional info
            stdout.flush()
        }
    }
}
pub enum Direction { Up, Down, Left, Right }
