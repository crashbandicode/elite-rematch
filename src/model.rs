//! Host-testable half: hook sites, the post-match popup records and what the
//! plugin does with them.
//!
//! After an online match the result component (`.text+0x147f960`) asks the
//! network state whether the local player's fighter just entered (`+0x180`)
//! or left (`+0x188`) Elite Smash ("VIP" in the game) and picks one of three
//! popup records. All three share one completion: button 2 is the rematch
//! (code 7, the session is reused), any other button leaves (code 1, the game
//! then re-classifies the fighter and switches between Quickplay and Elite
//! Smash). The record is parsed into a reusable descriptor at `.147fb60` and
//! shown at `.147fb68`. When the Elite Smash record leaves no rematch button,
//! the plugin parses the ordinary record over it before it is shown, keeping
//! the Elite Smash text as the indicator. Nothing else changes: the rating,
//! the classification, the network messages and the mode switch after the
//! last rematch stay native.

use core::fmt;

pub const SUPPORTED_VERSION: &str = "13.0.5";

/// An inline hook site and the 64 bytes that must be there (13.0.5 image).
pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub guard: [u8; 64],
}

/// After the record choice, before parsing: `ldr x8, [x19, #0x10]`.
/// x20 = the chosen record. Diagnostics only (forcing a record).
pub const SELECT_OFFSET: usize = 0x147faf0;
pub const SELECT: Site = Site {
    name: "popup_select",
    offset: SELECT_OFFSET,
    guard: [
        0x68, 0x0a, 0x40, 0xf9, 0x09, 0xc5, 0x40, 0xb9, 0x3f, 0x4d, 0x00, 0x71, 0x82, 0x06, 0x00,
        0x54, 0xea, 0x03, 0x00, 0x32, 0x0a, 0x00, 0x00, 0x14, 0x68, 0x0a, 0x40, 0xf9, 0x09, 0xc5,
        0x40, 0xb9, 0x3f, 0x4d, 0x00, 0x71, 0xc2, 0x05, 0x00, 0x54, 0x74, 0xc6, 0x95, 0xd2, 0x94,
        0x79, 0xa9, 0xf2, 0x74, 0xe3, 0xdf, 0xf2, 0xf4, 0x1f, 0xeb, 0xf2, 0xea, 0x07, 0x00, 0x32,
        0x08, 0xc9, 0x29, 0x8b,
    ],
};

/// After the native parse, before the popup is shown: `ldp x0, x1, [x19, #8]`.
/// x19 = the popup wrapper (`+0x10` the descriptor), x20 = the parsed record.
pub const PARSED_OFFSET: usize = 0x147fb64;
pub const PARSED: Site = Site {
    name: "popup_parsed",
    offset: PARSED_OFFSET,
    guard: [
        0x60, 0x86, 0x40, 0xa9, 0x12, 0xc8, 0x79, 0x94, 0xf3, 0x07, 0x40, 0xf9, 0xf3, 0xf6, 0xff,
        0xb5, 0xc2, 0xff, 0xff, 0x17, 0x68, 0x06, 0x40, 0xf9, 0x1f, 0x19, 0x01, 0xb9, 0xf3, 0x07,
        0x40, 0xf9, 0x53, 0xf6, 0xff, 0xb5, 0xbd, 0xff, 0xff, 0x17, 0xa0, 0xf4, 0x01, 0xb0, 0x00,
        0x60, 0x13, 0x91, 0xc7, 0x02, 0x95, 0x94, 0x60, 0xf2, 0xff, 0x34, 0x19, 0x91, 0x0b, 0x94,
        0x20, 0x17, 0x00, 0x90,
    ],
};

/// The popup's completion: `ldr w8, [x8, #0x118]`, x8 = the popup manager,
/// `+0x118` the chosen button (1-based). Read-only, for the log.
pub const CHOICE_OFFSET: usize = 0x147f4d0;
pub const CHOICE: Site = Site {
    name: "popup_choice",
    offset: CHOICE_OFFSET,
    guard: [
        0x08, 0x19, 0x41, 0xb9, 0x1f, 0x09, 0x00, 0x71, 0x61, 0x01, 0x00, 0x54, 0xe8, 0x0b, 0x00,
        0x32, 0x68, 0x62, 0x03, 0x39, 0x28, 0xf2, 0x01, 0xd0, 0x08, 0x79, 0x40, 0xf9, 0x08, 0x01,
        0x00, 0xb4, 0x00, 0x21, 0x16, 0x91, 0x08, 0xc5, 0x42, 0xf9, 0x08, 0x35, 0x40, 0xf9, 0x00,
        0x01, 0x3f, 0xd6, 0x03, 0x00, 0x00, 0x14, 0xe8, 0x03, 0x00, 0x32, 0x68, 0x62, 0x03, 0x39,
        0x68, 0xc6, 0x80, 0xb9,
    ],
};

/// Native popup parser `(descriptor, record)`, the call at `.147fb60`.
pub const PARSE_POPUP_OFFSET: usize = 0x32edf70;

/// Popup records chosen at `.147faa0..147faec`.
pub const POPUP_ORDINARY: u64 = 0x58ff_ff1c_b2b1_1288;
pub const POPUP_ENTERED: u64 = 0x58ff_ff19_3b38_def1;
pub const POPUP_LEFT: u64 = 0x58ff_ff17_f0f2_c4be;

/// Button the popup manager reports for the rematch (code 7 at `.147f4dc`).
pub const REMATCH_BUTTON: u32 = 2;

/// Descriptor layout written by the parser (`.32edf70`).
pub mod descriptor {
    pub const STYLE: usize = 0x0;
    /// Body text block: hash, length, 64-byte label.
    pub const BODY: usize = 0x1c;
    pub const BODY_END: usize = 0x64;
    pub const BODY_LABEL: usize = 0x24;
    pub const BUTTON_LABELS: [usize; 3] = [0x6c, 0xb4, 0xfc];
    pub const LABEL_LEN: usize = 64;
    pub const DEFAULT_BUTTON: usize = 0x160;
    pub const RECORD: usize = 0x198;

    // The saved body block holds the whole label and stops before the buttons.
    const _: () = assert!(BODY <= BODY_LABEL && BODY_LABEL + LABEL_LEN <= BODY_END);
    const _: () = assert!(BODY_END <= BUTTON_LABELS[0]);
}

/// The local player's Elite Smash change reported by the result component.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    Entered,
    Left,
}

impl Change {
    pub fn of(record: u64) -> Option<Self> {
        match record {
            POPUP_ENTERED => Some(Self::Entered),
            POPUP_LEFT => Some(Self::Left),
            _ => None,
        }
    }

    pub fn record(self) -> u64 {
        match self {
            Self::Entered => POPUP_ENTERED,
            Self::Left => POPUP_LEFT,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Entered => "entered",
            Self::Left => "left",
        }
    }
}

/// Text of a popup that keeps the rematch after an Elite Smash change.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Body {
    /// The game's own Elite Smash text, with the rematch buttons.
    Elite,
    /// The ordinary rematch popup unchanged.
    Rematch,
}

impl Body {
    pub fn name(self) -> &'static str {
        match self {
            Self::Elite => "elite",
            Self::Rematch => "rematch",
        }
    }
}

/// What the parsed-popup hook does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plan {
    /// Show the popup as parsed.
    Keep,
    /// Parse the ordinary record over it; with `keep_body`, put the Elite
    /// Smash text back afterwards.
    Reparse { change: Change, keep_body: bool },
}

pub fn plan(record: u64, retain: bool, body: Body) -> Plan {
    match Change::of(record) {
        Some(change) if retain => Plan::Reparse {
            change,
            keep_body: body == Body::Elite,
        },
        _ => Plan::Keep,
    }
}

/// Settings. The shipped build always uses `DEFAULT`; Eden-only diagnostic
/// builds read `diag.txt` (`force=entered|left`, `retain=0|1`,
/// `body=elite|rematch`, one per line).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    /// Replace the ordinary record with this one (diagnostics: an Elite
    /// Smash change without a server that can produce one).
    pub force: Option<Change>,
    pub retain: bool,
    pub body: Body,
}

impl Settings {
    pub const DEFAULT: Self = Self {
        force: None,
        retain: true,
        body: Body::Elite,
    };

    /// One word, so the hooks can read settings the writer thread reloads.
    pub fn to_bits(self) -> u32 {
        let force = match self.force {
            None => 0,
            Some(Change::Entered) => 1,
            Some(Change::Left) => 2,
        };
        force | (u32::from(self.retain) << 2) | (u32::from(self.body == Body::Rematch) << 3)
    }

    pub fn from_bits(bits: u32) -> Self {
        Self {
            force: match bits & 3 {
                1 => Some(Change::Entered),
                2 => Some(Change::Left),
                _ => None,
            },
            retain: bits & 4 != 0,
            body: if bits & 8 != 0 {
                Body::Rematch
            } else {
                Body::Elite
            },
        }
    }

    pub fn parse(text: &[u8]) -> Self {
        let mut settings = Self::DEFAULT;
        for line in text.split(|b| *b == b'\n') {
            let line = trim(line);
            let Some(eq) = line.iter().position(|b| *b == b'=') else {
                continue;
            };
            let (key, value) = (trim(&line[..eq]), trim(&line[eq + 1..]));
            match (key, value) {
                (b"force", b"entered") => settings.force = Some(Change::Entered),
                (b"force", b"left") => settings.force = Some(Change::Left),
                (b"force", b"none") => settings.force = None,
                (b"retain", b"0") => settings.retain = false,
                (b"retain", b"1") => settings.retain = true,
                (b"body", b"elite") => settings.body = Body::Elite,
                (b"body", b"rematch") => settings.body = Body::Rematch,
                _ => {}
            }
        }
        settings
    }
}

fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |i| i + 1);
    &bytes[start..end]
}

/// A label stored in the descriptor: the printable ASCII up to the first NUL.
pub fn label(bytes: &[u8]) -> &str {
    let end = bytes
        .iter()
        .position(|b| !(0x20..0x7f).contains(b))
        .unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..end]).unwrap_or("")
}

/// A text line built in a fixed buffer; anything past `N` bytes is dropped.
pub struct Line<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Line<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The text plus a NUL, for `nn::fs` paths; `None` when there is no room.
    pub fn with_nul(&mut self) -> Option<&[u8]> {
        if self.len >= N {
            return None;
        }
        self.buf[self.len] = 0;
        Some(&self.buf[..=self.len])
    }
}

impl<const N: usize> Default for Line<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Write for Line<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let take = s.len().min(N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write as _;

    #[test]
    fn guards_start_with_the_hooked_instruction() {
        assert_eq!(SELECT.guard[..4], [0x68, 0x0a, 0x40, 0xf9]); // ldr x8, [x19, #0x10]
        assert_eq!(PARSED.guard[..4], [0x60, 0x86, 0x40, 0xa9]); // ldp x0, x1, [x19, #8]
        assert_eq!(CHOICE.guard[..4], [0x08, 0x19, 0x41, 0xb9]); // ldr w8, [x8, #0x118]
    }

    #[test]
    fn records_map_to_changes() {
        assert_eq!(Change::of(POPUP_ORDINARY), None);
        assert_eq!(Change::of(POPUP_ENTERED), Some(Change::Entered));
        assert_eq!(Change::of(POPUP_LEFT), Some(Change::Left));
        assert_eq!(Change::of(0), None);
        for change in [Change::Entered, Change::Left] {
            assert_eq!(Change::of(change.record()), Some(change));
        }
    }

    #[test]
    fn only_elite_records_are_reparsed() {
        assert_eq!(plan(POPUP_ORDINARY, true, Body::Elite), Plan::Keep);
        assert_eq!(plan(0x58ff_ff1b_4bcc_ae33, true, Body::Elite), Plan::Keep);
        assert_eq!(
            plan(POPUP_ENTERED, true, Body::Elite),
            Plan::Reparse {
                change: Change::Entered,
                keep_body: true
            }
        );
        assert_eq!(
            plan(POPUP_LEFT, true, Body::Rematch),
            Plan::Reparse {
                change: Change::Left,
                keep_body: false
            }
        );
        assert_eq!(plan(POPUP_LEFT, false, Body::Elite), Plan::Keep);
    }

    #[test]
    fn settings_parse() {
        assert_eq!(Settings::parse(b""), Settings::DEFAULT);
        let s = Settings::parse(b"force = entered\r\nretain=0\nbody=rematch\njunk\n");
        assert_eq!(s.force, Some(Change::Entered));
        assert!(!s.retain);
        assert_eq!(s.body, Body::Rematch);
        assert_eq!(
            Settings::parse(b"force=left\nforce=none").force,
            None,
            "the last line wins"
        );
        assert_eq!(Settings::parse(b"force=sideways"), Settings::DEFAULT);
    }

    #[test]
    fn settings_round_trip_through_bits() {
        for force in [None, Some(Change::Entered), Some(Change::Left)] {
            for retain in [false, true] {
                for body in [Body::Elite, Body::Rematch] {
                    let s = Settings {
                        force,
                        retain,
                        body,
                    };
                    assert_eq!(Settings::from_bits(s.to_bits()), s);
                }
            }
        }
    }

    #[test]
    fn label_stops_at_nul_or_binary() {
        assert_eq!(label(b"mnu_online_x\0garbage"), "mnu_online_x");
        assert_eq!(label(b"\0"), "");
        assert_eq!(label(&[b'a', 0xff, b'b']), "a");
    }

    #[test]
    fn line_drops_overflow_and_adds_nul() {
        let mut line = Line::<8>::new();
        let _ = write!(line, "0123456789");
        assert_eq!(line.as_bytes(), b"01234567");
        assert!(line.with_nul().is_none());
        line.clear();
        let _ = write!(line, "ab");
        assert_eq!(line.with_nul(), Some(&b"ab\0"[..]));
    }
}
