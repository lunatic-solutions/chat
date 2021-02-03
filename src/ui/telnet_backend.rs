use std::{cell::RefCell, fmt, io::Write, rc::Rc};

use lunatic::net::TcpStream;
use tui::{
    backend::Backend,
    style::{Color, Modifier},
};

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
        use std::fmt::Write;

        let mut string = String::with_capacity(content.size_hint().0 * 3);
        let mut fg = Color::Reset;
        let mut bg = Color::Reset;
        let mut modifier = Modifier::empty();
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in content {
            // Move the cursor if the previous location was not (x - 1, y)
            if !matches!(last_pos, Some(p) if x == p.0 + 1 && y == p.1) {
                self.set_cursor(x, y).unwrap();
            }
            last_pos = Some((x, y));
            if cell.modifier != modifier {
                write!(
                    string,
                    "{}",
                    ModifierDiff {
                        from: modifier,
                        to: cell.modifier
                    }
                )
                .unwrap();
                modifier = cell.modifier;
            }
            if cell.fg != fg {
                write!(string, "{}", Fg(cell.fg)).unwrap();
                fg = cell.fg;
            }
            if cell.bg != bg {
                write!(string, "{}", Bg(cell.bg)).unwrap();
                bg = cell.bg;
            }
            string.push_str(&cell.symbol);
        }
        write!(
            self.tcp_stream,
            "{}{}{}{}",
            string,
            Fg(Color::Reset),
            Bg(Color::Reset),
            "\u{001B}[m",
        )
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

// The rest is taken from tui::backend::termion and modified with concrete value
struct Fg(Color);

struct Bg(Color);

struct ModifierDiff {
    from: Modifier,
    to: Modifier,
}

impl fmt::Display for Fg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Color::Reset => write!(f, "\u{001B}[39m"),
            // Color::Black => termion::color::Black.write_fg(f),
            // Color::Red => termion::color::Red.write_fg(f),
            // Color::Green => termion::color::Green.write_fg(f),
            // Color::Yellow => termion::color::Yellow.write_fg(f),
            // Color::Blue => termion::color::Blue.write_fg(f),
            // Color::Magenta => termion::color::Magenta.write_fg(f),
            // Color::Cyan => termion::color::Cyan.write_fg(f),
            // Color::Gray => termion::color::White.write_fg(f),
            // Color::DarkGray => termion::color::LightBlack.write_fg(f),
            // Color::LightRed => termion::color::LightRed.write_fg(f),
            // Color::LightGreen => termion::color::LightGreen.write_fg(f),
            // Color::LightBlue => termion::color::LightBlue.write_fg(f),
            // Color::LightYellow => termion::color::LightYellow.write_fg(f),
            // Color::LightMagenta => termion::color::LightMagenta.write_fg(f),
            // Color::LightCyan => termion::color::LightCyan.write_fg(f),
            // Color::White => termion::color::LightWhite.write_fg(f),
            // Color::Indexed(i) => termion::color::AnsiValue(i).write_fg(f),
            // Color::Rgb(r, g, b) => termion::color::Rgb(r, g, b).write_fg(f),
            _ => unimplemented!(),
        }
    }
}
impl fmt::Display for Bg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Color::Reset => write!(f, "\u{001B}[49m"),
            // Color::Black => termion::color::Black.write_bg(f),
            // Color::Red => termion::color::Red.write_bg(f),
            // Color::Green => termion::color::Green.write_bg(f),
            // Color::Yellow => termion::color::Yellow.write_bg(f),
            // Color::Blue => termion::color::Blue.write_bg(f),
            // Color::Magenta => termion::color::Magenta.write_bg(f),
            // Color::Cyan => termion::color::Cyan.write_bg(f),
            // Color::Gray => termion::color::White.write_bg(f),
            // Color::DarkGray => termion::color::LightBlack.write_bg(f),
            // Color::LightRed => termion::color::LightRed.write_bg(f),
            // Color::LightGreen => termion::color::LightGreen.write_bg(f),
            // Color::LightBlue => termion::color::LightBlue.write_bg(f),
            // Color::LightYellow => termion::color::LightYellow.write_bg(f),
            // Color::LightMagenta => termion::color::LightMagenta.write_bg(f),
            // Color::LightCyan => termion::color::LightCyan.write_bg(f),
            // Color::White => termion::color::LightWhite.write_bg(f),
            // Color::Indexed(i) => termion::color::AnsiValue(i).write_bg(f),
            // Color::Rgb(r, g, b) => termion::color::Rgb(r, g, b).write_bg(f),
            _ => unimplemented!(),
        }
    }
}

impl fmt::Display for ModifierDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let remove = self.from - self.to;
        if remove.contains(Modifier::REVERSED) {
            write!(f, "\u{001B}[27m")?;
        }
        if remove.contains(Modifier::BOLD) {
            // XXX: the termion NoBold flag actually enables double-underline on ECMA-48 compliant
            // terminals, and NoFaint additionally disables bold... so we use this trick to get
            // the right semantics.
            write!(f, "\u{001B}[22m")?;

            if self.to.contains(Modifier::DIM) {
                write!(f, "\u{001B}[2m")?;
            }
        }
        if remove.contains(Modifier::ITALIC) {
            write!(f, "\u{001B}[23m")?;
        }
        if remove.contains(Modifier::UNDERLINED) {
            write!(f, "\u{001B}[24m")?;
        }
        if remove.contains(Modifier::DIM) {
            write!(f, "\u{001B}[22m")?;

            // XXX: the NoFaint flag additionally disables bold as well, so we need to re-enable it
            // here if we want it.
            if self.to.contains(Modifier::BOLD) {
                write!(f, "{}", "\u{001B}[1m")?;
            }
        }
        if remove.contains(Modifier::CROSSED_OUT) {
            write!(f, "{}", "\u{001B}[9m")?;
        }
        if remove.contains(Modifier::SLOW_BLINK) || remove.contains(Modifier::RAPID_BLINK) {
            write!(f, "{}", "\u{001B}[25m")?;
        }

        let add = self.to - self.from;
        if add.contains(Modifier::REVERSED) {
            write!(f, "\u{001B}[7m")?;
        }
        if add.contains(Modifier::BOLD) {
            write!(f, "\u{001B}[1m")?;
        }
        if add.contains(Modifier::ITALIC) {
            write!(f, "\u{001B}[3m")?;
        }
        if add.contains(Modifier::UNDERLINED) {
            write!(f, "\u{001B}[4m")?;
        }
        if add.contains(Modifier::DIM) {
            write!(f, "\u{001B}[2m")?;
        }
        if add.contains(Modifier::CROSSED_OUT) {
            write!(f, "\u{001B}[9m")?;
        }
        if add.contains(Modifier::SLOW_BLINK) || add.contains(Modifier::RAPID_BLINK) {
            write!(f, "\u{001B}[5m")?;
        }

        Ok(())
    }
}
