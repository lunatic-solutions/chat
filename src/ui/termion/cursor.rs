use numtoa::NumToA;
use std::fmt;
use std::io::{self, Write};
use std::ops;

/// Create a CSI-introduced sequence.
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

/// Derive a CSI sequence struct.
macro_rules! derive_csi_sequence {
    ($doc:expr, $name:ident, $value:expr) => {
        #[doc = $doc]
        #[derive(Copy, Clone)]
        pub struct $name;

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, csi!($value))
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &'static [u8] { csi!($value).as_bytes() }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &'static str { csi!($value) }
        }
    };
}

derive_csi_sequence!("Hide the cursor.", Hide, "?25l");
derive_csi_sequence!("Show the cursor.", Show, "?25h");

derive_csi_sequence!("Restore the cursor.", Restore, "u");
derive_csi_sequence!("Save the cursor.", Save, "s");

derive_csi_sequence!(
    "Change the cursor style to blinking block",
    BlinkingBlock,
    "\x31 q"
);
derive_csi_sequence!(
    "Change the cursor style to steady block",
    SteadyBlock,
    "\x32 q"
);
derive_csi_sequence!(
    "Change the cursor style to blinking underline",
    BlinkingUnderline,
    "\x33 q"
);
derive_csi_sequence!(
    "Change the cursor style to steady underline",
    SteadyUnderline,
    "\x34 q"
);
derive_csi_sequence!(
    "Change the cursor style to blinking bar",
    BlinkingBar,
    "\x35 q"
);
derive_csi_sequence!("Change the cursor style to steady bar", SteadyBar, "\x36 q");

/// Goto some position ((1,1)-based).
///
/// # Why one-based?
///
/// ANSI escapes are very poorly designed, and one of the many odd aspects is being one-based. This
/// can be quite strange at first, but it is not that big of an obstruction once you get used to
/// it.
///
/// # Example
///
/// ```rust
/// extern crate termion;
///
/// fn main() {
///     print!("{}{}Stuff", termion::clear::All, termion::cursor::Goto(5, 3));
/// }
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Goto(pub u16, pub u16);

impl From<Goto> for String {
    fn from(this: Goto) -> String {
        let (mut x, mut y) = ([0u8; 20], [0u8; 20]);
        [
            "\x1B[",
            this.1.numtoa_str(10, &mut x),
            ";",
            this.0.numtoa_str(10, &mut y),
            "H",
        ]
        .concat()
    }
}

impl Default for Goto {
    fn default() -> Goto {
        Goto(1, 1)
    }
}

impl fmt::Display for Goto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug_assert!(self != &Goto(0, 0), "Goto is one-based.");
        write!(f, "\x1B[{};{}H", self.1, self.0)
    }
}

/// Move cursor left.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Left(pub u16);

impl From<Left> for String {
    fn from(this: Left) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "D"].concat()
    }
}

impl fmt::Display for Left {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}D", self.0)
    }
}

/// Move cursor right.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Right(pub u16);

impl From<Right> for String {
    fn from(this: Right) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "C"].concat()
    }
}

impl fmt::Display for Right {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}C", self.0)
    }
}

/// Move cursor up.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Up(pub u16);

impl From<Up> for String {
    fn from(this: Up) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "A"].concat()
    }
}

impl fmt::Display for Up {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}A", self.0)
    }
}

/// Move cursor down.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Down(pub u16);

impl From<Down> for String {
    fn from(this: Down) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "B"].concat()
    }
}

impl fmt::Display for Down {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}B", self.0)
    }
}

/// Hide the cursor for the lifetime of this struct.
/// It will hide the cursor on creation with from() and show it back on drop().
pub struct HideCursor<W: Write> {
    /// The output target.
    output: W,
}

impl<W: Write> Drop for HideCursor<W> {
    fn drop(&mut self) {
        write!(self, "{}", Show).expect("show the cursor");
    }
}

impl<W: Write> ops::Deref for HideCursor<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: Write> ops::DerefMut for HideCursor<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: Write> Write for HideCursor<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}
