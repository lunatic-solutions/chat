pub mod telnet_backend;

use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    symbols::DOT,
    widgets::{Clear, Paragraph, Wrap},
};
use tui::{style::Color, terminal::Frame, Terminal};
use tui::{
    style::Style,
    text::Spans,
    widgets::{Block, Borders, Tabs, Widget},
};

use lunatic::net::TcpStream;
use telnet_backend::TelnetBackend;

pub struct Ui {
    terminal: Terminal<TelnetBackend>,
    tabs: Vec<(Spans<'static>, TabType)>,
    selected: usize,
}

impl Ui {
    pub fn new(tcp_stream: TcpStream, window_size: telnet_backend::WindowSize) -> Self {
        let backend = TelnetBackend::new(tcp_stream, window_size);
        let terminal = Terminal::new(backend).unwrap();
        Self {
            terminal,
            tabs: Vec::new(),
            selected: 0,
        }
    }

    pub fn add_tab(&mut self, name: String, tab: TabType) {
        self.tabs.push((name.into(), tab))
    }

    pub fn render(&mut self) {
        let tabs = self.tabs.iter().map(|pair| pair.0.clone()).collect();
        let selected_tab = self.tabs.get(self.selected).unwrap().1.clone();

        self.terminal
            .draw(|f| {
                let size = f.size();
                if size.width < 80 || size.height < 24 {
                    return Self::render_size_warning(f);
                }

                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Percentage(5),
                            Constraint::Percentage(85),
                            Constraint::Percentage(10),
                        ]
                        .as_ref(),
                    )
                    .split(size);

                // Render tabs
                let tabs = Tabs::new(tabs)
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().fg(Color::Yellow))
                    .divider(DOT)
                    .select(0);
                f.render_widget(tabs, layout[0]);

                // Render selected tab content
                match selected_tab {
                    TabType::Welcome(content) => Self::render_welcome(f, content, layout[1]),
                    _ => unimplemented!(),
                }

                // Render input box
            })
            .unwrap();
    }

    fn render_size_warning(frame: &mut Frame<TelnetBackend>) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(frame.size());
        let warning = Paragraph::new("Please resize your terminal window to at least: 80x24")
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(warning, layout[1]);
    }

    fn render_welcome(frame: &mut Frame<TelnetBackend>, content: String, area: Rect) {
        let welcome = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(welcome, area);
    }
}

#[derive(Clone)]
pub enum TabType {
    Welcome(String),
    Channel,
}
