pub mod telnet_backend;
pub mod termion;

use std::{cell::RefCell, mem, rc::Rc};

use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    symbols::{bar, DOT},
    text::Span,
    widgets::{Paragraph, Wrap},
};
use tui::{style::Color, terminal::Frame, Terminal};
use tui::{
    style::Style,
    text::Spans,
    widgets::{Block, Borders, Tabs},
};

use lunatic::{channel::Sender, net::TcpStream};
use telnet_backend::TelnetBackend;

use crate::channel::ChannelMessage;

pub struct Ui {
    terminal: Terminal<TelnetBackend>,
    tabs: UiTabs,
}

impl Ui {
    pub fn new(
        tcp_stream: TcpStream,
        window_size: telnet_backend::WindowSize,
        tabs: UiTabs,
    ) -> Self {
        let backend = TelnetBackend::new(tcp_stream, window_size);
        let terminal = Terminal::new(backend).unwrap();
        Self { terminal, tabs }
    }

    pub fn render(&mut self) {
        let tabs = self.tabs.widget();
        let selected_tab = self.tabs.get_selected();
        let _ = self.terminal.draw(|f| {
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
            f.render_widget(tabs, layout[0]);

            // Render selected tab content
            match selected_tab.get_type() {
                TabType::Info(content) => {
                    // Render selected tab content
                    Self::render_info(f, content, layout[1]);
                    // Render input box
                    Self::render_input(f, selected_tab.get_input(), layout[2])
                }
                TabType::Channel(content) => {
                    // Render channel
                    Self::render_channel(f, content, layout[1]);
                    // Render input box
                    Self::render_input(f, selected_tab.get_input(), layout[2])
                }
            }
        });
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

    fn render_info(frame: &mut Frame<TelnetBackend>, content: String, area: Rect) {
        let welcome = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(welcome, area);
    }

    fn render_channel(
        frame: &mut Frame<TelnetBackend>,
        content: Vec<(String, String, String)>,
        area: Rect,
    ) {
        let mut lines = Vec::with_capacity(content.len());
        // +2 to calculate boarders
        let mut vertical_space_used = 2;
        for line in content {
            let spans = Spans::from(vec![
                Span::styled(line.0, Style::default().fg(Color::Yellow)),
                Span::styled(line.1, Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(": ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(line.2),
            ]);
            let width = spans.width();
            lines.push(spans);
            // -2 for boarders, -1 to only add if overflown
            vertical_space_used += (width as i16 / (area.width - 3) as i16) + 1;
        }
        // Calculate scroll
        let scroll = vertical_space_used - area.height as i16 + 1; // 1 line as buffer
        let scroll = if scroll < 0 { 0 } else { scroll };

        let chat = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL))
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: true });
        frame.render_widget(chat, area);
    }

    fn render_input(frame: &mut Frame<TelnetBackend>, content: String, area: Rect) {
        let arrow_style = Style::default().add_modifier(Modifier::ITALIC);
        let arrow = Span::styled("> ", arrow_style);

        let content = Span::raw(content);

        let cursor_style = Style::default().add_modifier(Modifier::RAPID_BLINK);
        let cursor = Span::styled(bar::FULL, cursor_style);

        let input = Spans::from(vec![arrow, content, cursor]);
        let welcome = Paragraph::new(input)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(welcome, area);
    }
}

#[derive(Clone)]
pub struct UiTabs {
    inner: Rc<RefCell<UiTabsInner>>,
}

struct UiTabsInner {
    tabs: Vec<Tab>,
    selected: usize,
}

impl UiTabs {
    pub fn new(tab: Tab) -> Self {
        let inner = UiTabsInner {
            tabs: vec![tab],
            selected: 0,
        };
        Self {
            inner: Rc::new(RefCell::new(inner)),
        }
    }

    pub fn widget(&self) -> Tabs {
        let immutable = self.inner.as_ref().borrow();
        let tabs = immutable
            .tabs
            .iter()
            .map(|tab| Spans::from(tab.get_name()))
            .collect();
        Tabs::new(tabs)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider(DOT)
            .select(immutable.selected)
    }

    pub fn add(&self, tab: Tab) {
        let mut mutable = self.inner.as_ref().borrow_mut();
        mutable.tabs.push(tab);
        mutable.selected = mutable.tabs.len() - 1;
    }

    pub fn drop(&self) {
        let mut mutable = self.inner.as_ref().borrow_mut();
        // Don't drop the last tab
        if mutable.tabs.len() == 1 {
            return;
        }
        let index = mutable.selected;
        mutable.tabs.remove(index).drop();
        if index != 0 {
            mutable.selected -= 1;
        }
    }

    pub fn add_message(&self, channel: String, timestamp: String, user: String, message: String) {
        let mut mutable = self.inner.as_ref().borrow_mut();
        let tab = mutable
            .tabs
            .iter_mut()
            .find(|tab| tab.name == channel)
            .unwrap();
        match &mut tab.tab_type {
            TabType::Channel(content) => {
                content.push((timestamp, user, message));
                if content.len() > 100 {
                    content.drain(0..50);
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn next(&self) {
        let mut mutable = self.inner.as_ref().borrow_mut();
        mutable.selected = (mutable.selected + 1) % mutable.tabs.len();
    }

    pub fn get_selected(&self) -> Tab {
        let immutable = self.inner.as_ref().borrow();
        let index = immutable.selected;
        immutable.tabs.get(index).unwrap().clone()
    }

    pub fn input_del_char(&mut self) {
        let mut mutable = self.inner.as_ref().borrow_mut();
        let index = mutable.selected;
        let selected = mutable.tabs.get_mut(index).unwrap();
        selected.input_del_char();
    }

    pub fn input_add_char(&mut self, ch: char) {
        let mut mutable = self.inner.as_ref().borrow_mut();
        let index = mutable.selected;
        let selected = mutable.tabs.get_mut(index).unwrap();
        selected.input_add_char(ch);
    }

    pub fn clear(&mut self) -> String {
        let mut mutable = self.inner.as_ref().borrow_mut();
        let index = mutable.selected;
        let selected = mutable.tabs.get_mut(index).unwrap();
        selected.clear()
    }

    pub fn names(&self) -> Vec<String> {
        let immutable = self.inner.as_ref().borrow();
        immutable.tabs.iter().map(|tab| tab.name.clone()).collect()
    }
}

#[derive(Clone)]
pub struct Tab {
    name: String,
    tab_type: TabType,
    notifier: Option<(u32, Sender<ChannelMessage>)>,
    input: String,
}

impl Tab {
    pub fn new(
        name: String,
        notifier: Option<(u32, Sender<ChannelMessage>)>,
        tab_type: TabType,
    ) -> Self {
        Self {
            name,
            tab_type,
            notifier,
            input: String::new(),
        }
    }

    pub fn get_type(&self) -> TabType {
        self.tab_type.clone()
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_input(&self) -> String {
        self.input.clone()
    }

    pub fn clear(&mut self) -> String {
        mem::replace(&mut self.input, String::new())
    }

    pub fn input_del_char(&mut self) {
        self.input.pop();
    }

    pub fn input_add_char(&mut self, ch: char) {
        self.input.push(ch);
    }

    pub fn message(&self, timestamp: String, user: String, message: String) {
        if let Some((_id, notifier)) = &self.notifier {
            notifier
                .send(ChannelMessage::Message(
                    "".to_string(),
                    timestamp,
                    user,
                    message,
                ))
                .unwrap();
        }
    }

    pub fn drop(&self) {
        if let Some((id, notifier)) = &self.notifier {
            notifier.send(ChannelMessage::Unsubscribe(*id)).unwrap();
        }
    }
}

#[derive(Clone)]
pub enum TabType {
    Info(String),
    Channel(Vec<(String, String, String)>),
}
