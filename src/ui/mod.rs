pub mod telnet_backend;

use tui::widgets::{Block, Borders, Widget};
use tui::Terminal;
use tui::{
    layout::{Constraint, Direction, Layout},
    widgets::Clear,
};

use lunatic::net::TcpStream;
use telnet_backend::TelnetBackend;

pub struct Ui {
    terminal: Terminal<TelnetBackend>,
}

impl Ui {
    pub fn new(tcp_stream: TcpStream, window_size: telnet_backend::WindowSize) -> Self {
        let backend = TelnetBackend::new(tcp_stream, window_size);
        let terminal = Terminal::new(backend).unwrap();
        Self { terminal }
    }

    pub fn render(&mut self) {
        self.terminal
            .draw(|f| {
                let size = f.size();
                let block = Block::default().title("Block").borders(Borders::ALL);
                f.render_widget(Clear, size);
                f.render_widget(block, size);
            })
            .unwrap();
    }
}
