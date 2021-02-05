use itertools::Itertools;
use lunatic::net::TcpStream;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

const IAC: u8 = 255;

const WILL: u8 = 251;
const WONT: u8 = 252;
const DO: u8 = 253;
const DONT: u8 = 254;

const SE: u8 = 240;
const SB: u8 = 250;

const ECHO: u8 = 1;
const LINEMODE: u8 = 34;
const NAWS: u8 = 31;

pub struct Telnet {
    stream: TcpStream,
    start: usize,
    end: usize,
    buffer: [u8; 1024],
}

impl Telnet {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: [0; 1024],
            start: 0,
            end: 0,
        }
    }

    pub fn iac_do_linemode(&mut self) -> Result<(), ()> {
        let buffer: [u8; 3] = [IAC, DO, LINEMODE];
        self.stream.write(&buffer).unwrap();
        self.stream.flush().unwrap();

        loop {
            match self.next()? {
                TelnetMessage::IacWillLinemode => return Ok(()),
                TelnetMessage::IacWontLinemode => return Err(()),
                _ => {}
            }
        }
    }

    // Tell the client not to do local editing
    pub fn iac_linemode_zero(&mut self) {
        let buffer: [u8; 7] = [IAC, SB, LINEMODE, 1, 0, IAC, SE];
        self.stream.write(&buffer).unwrap();
    }

    // Tell the client to report window size changes
    pub fn iac_do_naws(&mut self) -> Result<(), ()> {
        let buffer: [u8; 3] = [IAC, DO, NAWS];
        self.stream.write(&buffer).unwrap();

        loop {
            match self.next()? {
                TelnetMessage::IacWillNaws => return Ok(()),
                TelnetMessage::IacWontNaws => return Err(()),
                _ => {}
            }
        }
    }

    // Tell the client that we will be doing the echoing
    pub fn iac_will_echo(&mut self) -> Result<(), ()> {
        let buffer: [u8; 3] = [IAC, WILL, ECHO];
        self.stream.write(&buffer).unwrap();

        loop {
            match self.next()? {
                TelnetMessage::IacDoEcho => return Ok(()),
                TelnetMessage::IacDontEcho => return Err(()),
                _ => {}
            }
        }
    }

    /// Get next message from client
    pub fn next(&mut self) -> Result<TelnetMessage, ()> {
        // If we reached the end of the buffer read more from tcp stream
        if self.start == self.end {
            match self.stream.read(&mut self.buffer).unwrap() {
                0 => return Err(()),
                size => {
                    self.start = 0;
                    self.end = size;
                }
            }
        }

        let result = match self.buffer.get(self.start..self.end).unwrap() {
            [IAC, WILL, LINEMODE, ..] => {
                self.start += 3;
                TelnetMessage::IacWillLinemode
            }
            [IAC, WONT, LINEMODE, ..] => {
                self.start += 3;
                TelnetMessage::IacWontLinemode
            }
            [IAC, WILL, NAWS, ..] => {
                self.start += 3;
                TelnetMessage::IacWillNaws
            }
            [IAC, WONT, NAWS, ..] => {
                self.start += 3;
                TelnetMessage::IacWontNaws
            }
            [IAC, DO, ECHO, ..] => {
                self.start += 3;
                TelnetMessage::IacDoEcho
            }
            [IAC, DONT, ECHO, ..] => {
                self.start += 3;
                TelnetMessage::IacDontEcho
            }
            // Ignore other 3 byte patterns
            [IAC, DO | DONT | WILL | WONT, _, ..] => {
                self.start += 3;
                TelnetMessage::IacOther
            }
            // Handle NAWS
            multibyte @ [IAC, SB, NAWS, .., IAC, SE] => {
                let len = multibyte.len();
                let (width, height) = if len == 9 {
                    // If there are no double 255s
                    (
                        u16::from_be_bytes([multibyte[3], multibyte[4]]),
                        u16::from_be_bytes([multibyte[5], multibyte[6]]),
                    )
                } else {
                    // First deduplicate 255 values
                    let slice = multibyte.get(3..len - 2).unwrap();
                    let vec: Vec<&u8> = slice
                        .into_iter()
                        .dedup_by(|first, second| **first == 255 && **second == 255)
                        .collect();
                    (
                        u16::from_be_bytes([*vec[0], *vec[1]]),
                        u16::from_be_bytes([*vec[2], *vec[3]]),
                    )
                };
                self.start += len;
                TelnetMessage::Naws(width, height)
            }
            // Ignore multibyte SB patterns
            multibyte @ [IAC, SB, .., IAC, SE] => {
                self.start += multibyte.len();
                TelnetMessage::IacOther
            }
            // Escape characters
            [0x1b, 0x5b, esc, ..] => {
                self.start += 3;
                match esc {
                    65 => TelnetMessage::Up,
                    66 => TelnetMessage::Down,
                    67 => TelnetMessage::Right,
                    68 => TelnetMessage::Left,
                    _ => TelnetMessage::Ignore,
                }
            }
            // Enter (NL CR)
            [13, 0, ..] => {
                self.start += 2;
                TelnetMessage::Enter
            }
            [ch, ..] => {
                self.start += 1;
                match ch {
                    3 => TelnetMessage::CtrlC,
                    127 => TelnetMessage::Backspace,
                    9 => TelnetMessage::Tab,
                    27 => TelnetMessage::Esc,
                    _ => TelnetMessage::Char(*ch),
                }
            }
            [] => TelnetMessage::Error,
        };
        Ok(result)
    }
}

#[derive(Serialize, Deserialize)]
pub enum TelnetMessage {
    IacWillLinemode,
    IacWontLinemode,
    IacDoEcho,
    IacDontEcho,
    IacWillNaws,
    IacWontNaws,
    IacOther,
    Naws(u16, u16),
    Char(u8),
    Backspace,
    Enter,
    CtrlC,
    Tab,
    Esc,
    Up,
    Down,
    Right,
    Left,
    Ignore,
    Error,
}
