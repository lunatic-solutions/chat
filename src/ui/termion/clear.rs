use std::fmt;

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

derive_csi_sequence!("Clear the entire screen.", All, "2J");
derive_csi_sequence!("Clear everything after the cursor.", AfterCursor, "J");
derive_csi_sequence!("Clear everything before the cursor.", BeforeCursor, "1J");
derive_csi_sequence!("Clear the current line.", CurrentLine, "2K");
derive_csi_sequence!("Clear from cursor to newline.", UntilNewline, "K");
