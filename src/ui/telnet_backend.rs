use std::{borrow::BorrowMut, cell::RefCell, io::Write, rc::Rc, slice::Windows};

use lunatic::net::TcpStream;
use tui::backend::Backend;

#[derive(Clone)]
pub struct WindowSize {
    inner: Rc<RefCell<(u16, u16)>>,
}

impl WindowSize {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new((0, 0))),
        }
    }

    pub fn set(&self, width: u16, height: u16) {
        let mut size = self.inner.as_ref().borrow_mut();
        size.0 = width;
        size.1 = height;
    }

    pub fn get(&self) -> (u16, u16) {
        let size = self.inner.as_ref().borrow();
        (size.0, size.1)
    }
}

pub struct TelnetBackend {
    tcp_stream: TcpStream,
    window_size: WindowSize,
}

impl TelnetBackend {
    pub fn new(mut tcp_stream: TcpStream, window_size: WindowSize) -> Self {
        // Start at top left always
        write!(tcp_stream, "\u{001B}[{};{}H", 0, 0).unwrap();
        Self {
            tcp_stream,
            window_size,
        }
    }
}

impl Backend for TelnetBackend {
    fn draw<'a, I>(&mut self, content: I) -> Result<(), std::io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a tui::buffer::Cell)>,
    {
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in content {
            // Move the cursor if the previous location was not (x - 1, y)
            if !matches!(last_pos, Some(p) if x == p.0 + 1 && y == p.1) {
                self.set_cursor(x, y).unwrap();
            }
            last_pos = Some((x, y));
            self.tcp_stream.write(cell.symbol.as_bytes()).unwrap();
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), std::io::Error> {
        write!(self.tcp_stream, "\u{001B}[?25l").unwrap();
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), std::io::Error> {
        write!(self.tcp_stream, "\u{001B}[?25h").unwrap();
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), std::io::Error> {
        println!("GETTING CURSOR");
        Ok((0, 0))
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), std::io::Error> {
        write!(self.tcp_stream, "\u{001B}[{};{}H", y + 1, x + 1).unwrap();
        Ok(())
    }

    fn clear(&mut self) -> Result<(), std::io::Error> {
        write!(self.tcp_stream, "\u{001B}[2J").unwrap();
        self.set_cursor(0, 0).unwrap();
        Ok(())
    }

    fn size(&self) -> Result<tui::layout::Rect, std::io::Error> {
        let (width, height) = self.window_size.get();
        Ok(tui::layout::Rect::new(0, 0, width, height))
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.tcp_stream.flush()
    }
}
