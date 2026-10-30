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

/// A hook site and the 64 bytes there in the 13.0.5 image (`entry` says
/// which other plugins' hooks on them are accepted).
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

/// Message lookup `(manager, label) -> UTF-16 text`: every UI text is fetched
/// by label through it (e.g. `.37a26fc`, the label setter used by popups).
pub const MESSAGE_LOOKUP_OFFSET: usize = 0x37a4450;
pub const MESSAGE_LOOKUP: Site = Site {
    name: "message_lookup",
    offset: MESSAGE_LOOKUP_OFFSET,
    guard: [
        0xf6, 0x57, 0xbd, 0xa9, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00,
        0x91, 0x61, 0x07, 0x00, 0xb4, 0x28, 0x00, 0x40, 0x39, 0xf3, 0x03, 0x01, 0xaa, 0x08, 0x07,
        0x00, 0x34, 0x16, 0x0c, 0x40, 0xf9, 0xf4, 0x03, 0x00, 0xaa, 0x15, 0x80, 0x00, 0x91, 0x02,
        0x00, 0x00, 0x14, 0xf6, 0x03, 0x09, 0xaa, 0xdf, 0x02, 0x15, 0xeb, 0xe0, 0x02, 0x00, 0x54,
        0xc0, 0x46, 0x40, 0xf9,
    ],
};

/// Elite Smash classifier `.17049d0` (diagnostics only): at `.1704a98`
/// (`cmp w0, w19`) w0 = the threshold (parameter 1201), w19 = the fighter's
/// cached rating, x20 = the fighter's Hash40; the result is rating >=
/// threshold. Overriding w0 there makes a real GSP update cross a chosen
/// threshold in Eden, which has no rating server.
pub const CLASSIFY_OFFSET: usize = 0x1704a98;
pub const CLASSIFY: Site = Site {
    name: "classify",
    offset: CLASSIFY_OFFSET,
    guard: [
        0x1f, 0x00, 0x13, 0x6b, 0xe0, 0x87, 0x9f, 0x1a, 0xf4, 0xff, 0xff, 0x17, 0xeb, 0xf6, 0x8a,
        0x94, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x0f, 0x1b, 0xf8, 0xf8, 0x5f,
        0x01, 0xa9, 0xf6, 0x57, 0x02, 0xa9, 0xf4, 0x4f, 0x03, 0xa9, 0xfd, 0x7b, 0x04, 0xa9, 0xfd,
        0x03, 0x01, 0x91, 0x08, 0xde, 0x01, 0x90, 0x08, 0x2d, 0x46, 0xf9, 0x08, 0x05, 0x40, 0xf9,
        0xf3, 0x03, 0x00, 0xaa,
    ],
};

/// The classifier's "Elite Smash enabled" check (parameter 9010 == 1) at
/// `.1704a04` (`cmp w8, #1`), diagnostics only: logged, never changed.
pub const VIP_ENABLED_OFFSET: usize = 0x1704a04;
pub const VIP_ENABLED: Site = Site {
    name: "vip_enabled",
    offset: VIP_ENABLED_OFFSET,
    guard: [
        0x1f, 0x05, 0x00, 0x71, 0x21, 0x03, 0x00, 0x54, 0x48, 0xe1, 0x01, 0xf0, 0x08, 0x99, 0x43,
        0xf9, 0x08, 0x05, 0x40, 0xf9, 0x00, 0xb5, 0x40, 0xf9, 0xe1, 0x03, 0x14, 0xaa, 0x14, 0x7e,
        0x6d, 0x94, 0xe8, 0x03, 0x00, 0x2a, 0xe0, 0x03, 0x1f, 0x2a, 0x28, 0x02, 0xf8, 0x37, 0xe9,
        0x03, 0x28, 0x2a, 0x3f, 0x1d, 0x00, 0x72, 0xc0, 0x01, 0x00, 0x54, 0x08, 0x1d, 0x00, 0x12,
        0x1f, 0x8d, 0x01, 0x71,
    ],
};

/// Matchmaking search setup, read only: at `.168a5b8` (`str w0, [x25,
/// #0x8c]`) w0 = parameter 1199, the seconds an Elite Smash search waits
/// before it is restarted with the VIP filter widened to 0..1; x25 = network
/// context + 0x269c4, whose u32 at -4 is the search mode (1 Quickplay, 3
/// Elite Smash). It runs when a new search starts, not for a rematch (Eden,
/// 2026-10-30), so the plugin uses it as the start of a new lobby.
pub const SEARCH_DELAY_OFFSET: usize = 0x168a5b8;
pub const SEARCH_DELAY: Site = Site {
    name: "search_delay",
    offset: SEARCH_DELAY_OFFSET,
    guard: [
        0x20, 0x8f, 0x00, 0xb9, 0xc0, 0x5a, 0x40, 0xf9, 0x95, 0x02, 0x40, 0xb9, 0x81, 0x89, 0x80,
        0x52, 0xd6, 0xba, 0x02, 0x94, 0x60, 0x0c, 0x00, 0x34, 0x00, 0x03, 0x23, 0x1e, 0x01, 0x10,
        0x2e, 0x1e, 0x00, 0x28, 0x21, 0x1e, 0x01, 0x00, 0x22, 0x1e, 0x00, 0x18, 0x21, 0x1e, 0xbf,
        0x0e, 0x00, 0x71, 0xe0, 0x0b, 0x00, 0x54, 0xbf, 0x0a, 0x00, 0x71, 0xa0, 0x0c, 0x00, 0x54,
        0xbf, 0x06, 0x00, 0x71,
    ],
};

/// Instructions read at a site: its 64 guard bytes.
pub const SITE_WORDS: usize = 16;

impl Site {
    /// The 13.0.5 instruction `index` words from the site.
    pub fn stock(&self, index: usize) -> u32 {
        let b = &self.guard[index * 4..index * 4 + 4];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }
}

/// Skyline's far jump, `ldr x17, #8; br x17`, then the 8-byte target; a
/// `nop` comes first where the target would not be 8-byte aligned.
pub const FAR_JUMP: [u32; 2] = [0x5800_0051, 0xd61f_0220];
const NOP: u32 = 0xd503_201f;

/// Words a Skyline far jump at `address` takes (the most a hook replaces).
pub fn hooked_words(address: u64) -> usize {
    if address.is_multiple_of(8) {
        4
    } else {
        5
    }
}

/// A Skyline far jump at the start of `words` (the first one at `address`):
/// its length in words and, when all of it is in `words`, its target.
pub fn far_jump(words: &[u32], address: u64) -> Option<(usize, Option<u64>)> {
    let start = hooked_words(address) - 4;
    if start == 1 && words.first() != Some(&NOP) {
        return None;
    }
    if words.get(start..start + 2)? != FAR_JUMP {
        return None;
    }
    let target = words
        .get(start + 2..start + 4)
        .map(|w| u64::from(w[0]) | (u64::from(w[1]) << 32));
    Some((start + 4, target))
}

/// The target of a `b` at `address`.
pub fn branch(word: u32, address: u64) -> Option<u64> {
    (word & 0xfc00_0000 == 0x1400_0000)
        .then(|| address.wrapping_add_signed(i64::from(((word << 6) as i32) >> 4)))
}

/// What holds a hook site when the plugin loads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// The 13.0.5 instructions. Other plugins' hooks may sit further on,
    /// past the words this plugin's hook replaces.
    Stock,
    /// Another plugin's far jump over the site, to this handler, which runs
    /// the instructions the jump replaced: this plugin's hook continues into
    /// it (the jump's address word must not be relocated as code).
    Chain(u64),
    /// Another plugin's near branch over the first word, to this address.
    Branch(u64),
    /// Anything else: the site is left alone.
    Foreign,
}

impl Entry {
    pub fn name(self) -> &'static str {
        match self {
            Self::Stock => "stock",
            Self::Chain(_) => "chain",
            Self::Branch(_) => "branch",
            Self::Foreign => "foreign",
        }
    }
}

/// Reads the site's `words` (at `address`) against its 13.0.5 guard.
pub fn entry(site: &Site, words: &[u32; SITE_WORDS], address: u64) -> Entry {
    let (entry, mut i) = match far_jump(words, address) {
        Some((len, Some(target))) => {
            let end = address + 4 * len as u64;
            if target == 0 || !target.is_multiple_of(4) || (address..end).contains(&target) {
                return Entry::Foreign;
            }
            (Entry::Chain(target), len)
        }
        Some((_, None)) => return Entry::Foreign,
        None => match branch(words[0], address) {
            Some(target) if words[0] != site.stock(0) => (Entry::Branch(target), 1),
            _ => (Entry::Stock, 0),
        },
    };
    let replaced = hooked_words(address);
    while i < SITE_WORDS {
        let at = address + 4 * i as u64;
        if words[i] == site.stock(i) {
            i += 1;
        } else if i < replaced {
            return Entry::Foreign;
        } else if let Some((len, _)) = far_jump(&words[i..], at) {
            i += len;
        } else if branch(words[i], at).is_some() {
            i += 1;
        } else {
            return Entry::Foreign;
        }
    }
    entry
}

/// `foreign=1` in `diag.txt`: before its own hooks, the plugin hooks each
/// release site the way another plugin would, to test chaining onto it.
pub fn parse_foreign(text: &[u8]) -> bool {
    text.split(|b| *b == b'\n').fold(false, |on, line| {
        match line.iter().position(|b| *b == b'=') {
            Some(eq) if trim(&line[..eq]) == b"foreign" => trim(&line[eq + 1..]) == b"1",
            _ => on,
        }
    })
}

/// `threshold=<decimal>` in `diag.txt` (the last such line wins);
/// `threshold=native` or none keeps the game's parameter.
pub fn parse_threshold(text: &[u8]) -> Option<u32> {
    let mut threshold = None;
    for line in text.split(|b| *b == b'\n') {
        let Some(eq) = line.iter().position(|b| *b == b'=') else {
            continue;
        };
        if trim(&line[..eq]) != b"threshold" {
            continue;
        }
        let value = trim(&line[eq + 1..]);
        threshold = if value == b"native" {
            None
        } else {
            core::str::from_utf8(value)
                .ok()
                .and_then(|v| v.parse().ok())
                .or(threshold)
        };
    }
    threshold
}

/// Message labels of the three popup bodies (NUL-terminated, as the game
/// passes them to the lookup).
pub const LABEL_REMATCH: &[u8] = b"pop_result_battle_again\0";
pub const LABEL_ENTERED: &[u8] = b"pop_online_vip_start\0";
pub const LABEL_LEFT: &[u8] = b"pop_online_vip_end\0";
/// Longest combined text kept (UTF-16 units, NUL included).
pub const COMBINED_LEN: usize = 384;

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

    /// The Elite Smash notice's message label.
    pub fn label(self) -> &'static [u8] {
        match self {
            Self::Entered => LABEL_ENTERED,
            Self::Left => LABEL_LEFT,
        }
    }

    /// 1 or 2, for an atomic; 0 is "none".
    pub fn code(self) -> u32 {
        match self {
            Self::Entered => 1,
            Self::Left => 2,
        }
    }

    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::Entered),
            2 => Some(Self::Left),
            _ => None,
        }
    }
}

/// Whether the NUL-terminated label whose bytes `byte_at` returns equals
/// `label` (which includes its NUL). Reads no byte past the first mismatch
/// or the NUL.
pub fn label_matches(byte_at: impl Fn(usize) -> u8, label: &[u8]) -> bool {
    for (i, want) in label.iter().enumerate() {
        let got = byte_at(i);
        if got != *want {
            return false;
        }
        if got == 0 {
            return true;
        }
    }
    false
}

/// The notice, a line break and the rematch question, as one UTF-16 text with
/// a NUL. Both inputs stop at their first NUL; the result is cut to fit `out`
/// (the question first loses its tail, then the notice). Returns the units
/// written before the NUL.
pub fn combine(notice: &[u16], question: &[u16], out: &mut [u16]) -> usize {
    let until_nul = |text: &[u16]| text.iter().position(|u| *u == 0).unwrap_or(text.len());
    let (notice, question) = (
        &notice[..until_nul(notice)],
        &question[..until_nul(question)],
    );
    let Some(room) = out.len().checked_sub(1) else {
        return 0;
    };
    let mut len = notice.len().min(room);
    out[..len].copy_from_slice(&notice[..len]);
    if len < room && !question.is_empty() {
        out[len] = u16::from(b'\n');
        len += 1;
        let take = question.len().min(room - len);
        out[len..len + take].copy_from_slice(&question[..take]);
        len += take;
    }
    out[len] = 0;
    len
}

/// Text of a popup that keeps the rematch after an Elite Smash change.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Body {
    /// The game's Elite Smash notice, then its "Rematch?", one text.
    Both,
    /// The game's own Elite Smash text alone, with the rematch buttons.
    Elite,
    /// The ordinary rematch popup unchanged.
    Rematch,
}

impl Body {
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
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

/// `notice_shown`: the Elite Smash notice was already shown in this lobby.
/// A rematch stays in the first match's session, so the game keeps choosing
/// the notice after every rematch; only the first one shows it, later ones
/// are the plain "Rematch?".
pub fn plan(record: u64, retain: bool, body: Body, notice_shown: bool) -> Plan {
    match Change::of(record) {
        Some(change) if retain => Plan::Reparse {
            change,
            keep_body: body != Body::Rematch && !notice_shown,
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
        body: Body::Both,
    };

    /// One word, so the hooks can read settings the writer thread reloads.
    pub fn to_bits(self) -> u32 {
        let force = self.force.map_or(0, Change::code);
        let body = match self.body {
            Body::Both => 0,
            Body::Elite => 1,
            Body::Rematch => 2,
        };
        force | (u32::from(self.retain) << 2) | (body << 3)
    }

    pub fn from_bits(bits: u32) -> Self {
        Self {
            force: Change::from_code(bits & 3),
            retain: bits & 4 != 0,
            body: match (bits >> 3) & 3 {
                1 => Body::Elite,
                2 => Body::Rematch,
                _ => Body::Both,
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
                (b"body", b"both") => settings.body = Body::Both,
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
        assert_eq!(MESSAGE_LOOKUP.guard[..4], [0xf6, 0x57, 0xbd, 0xa9]); // stp x22, x21, [sp, #-0x30]!
        assert_eq!(SELECT.guard[..4], [0x68, 0x0a, 0x40, 0xf9]); // ldr x8, [x19, #0x10]
        assert_eq!(PARSED.guard[..4], [0x60, 0x86, 0x40, 0xa9]); // ldp x0, x1, [x19, #8]
        assert_eq!(CHOICE.guard[..4], [0x08, 0x19, 0x41, 0xb9]); // ldr w8, [x8, #0x118]
    }

    fn stock_words(site: &Site) -> [u32; SITE_WORDS] {
        core::array::from_fn(|i| site.stock(i))
    }

    fn far(target: u64) -> [u32; 4] {
        [
            FAR_JUMP[0],
            FAR_JUMP[1],
            target as u32,
            (target >> 32) as u32,
        ]
    }

    /// `b` from `from` to `to`.
    fn b(from: u64, to: u64) -> u32 {
        0x1400_0000 | ((to.wrapping_sub(from) >> 2) as u32 & 0x03ff_ffff)
    }

    const AT8: u64 = 0x80_0147_f4d0; // 8-byte aligned, like CHOICE
    const AT4: u64 = 0x80_0147_fb64; // 4 past, like PARSED
    const HANDLER: u64 = 0x71_2345_6780;

    #[test]
    fn stock_sites_are_stock() {
        for (site, at) in [(&CHOICE, AT8), (&PARSED, AT4), (&MESSAGE_LOOKUP, AT8)] {
            assert_eq!(entry(site, &stock_words(site), at), Entry::Stock);
        }
    }

    #[test]
    fn another_plugins_far_jump_is_chained() {
        let mut words = stock_words(&CHOICE);
        words[..4].copy_from_slice(&far(HANDLER));
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Chain(HANDLER));
        // Not 8-byte aligned: Skyline puts a nop first.
        let mut words = stock_words(&PARSED);
        words[0] = NOP;
        words[1..5].copy_from_slice(&far(HANDLER));
        assert_eq!(entry(&PARSED, &words, AT4), Entry::Chain(HANDLER));
        // That form without its nop is not Skyline's.
        let mut words = stock_words(&PARSED);
        words[..4].copy_from_slice(&far(HANDLER));
        assert_eq!(entry(&PARSED, &words, AT4), Entry::Foreign);
    }

    #[test]
    fn far_jumps_to_nowhere_or_into_themselves_are_foreign() {
        for target in [0, HANDLER + 2, AT8, AT8 + 8, AT8 + 12] {
            let mut words = stock_words(&CHOICE);
            words[..4].copy_from_slice(&far(target));
            assert_eq!(entry(&CHOICE, &words, AT8), Entry::Foreign, "{target:#x}");
        }
        let mut words = stock_words(&CHOICE);
        words[..4].copy_from_slice(&far(AT8 + 16));
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Chain(AT8 + 16));
    }

    #[test]
    fn another_plugins_branch_is_found_both_ways() {
        for to in [AT8 + 0x40_0000, AT8 - 0x40_0000] {
            let mut words = stock_words(&CHOICE);
            words[0] = b(AT8, to);
            assert_eq!(branch(words[0], AT8), Some(to));
            assert_eq!(entry(&CHOICE, &words, AT8), Entry::Branch(to));
        }
    }

    #[test]
    fn changes_in_the_replaced_words_are_foreign() {
        for i in 0..hooked_words(AT8) {
            let mut words = stock_words(&CHOICE);
            words[i] ^= 0x10;
            assert_eq!(entry(&CHOICE, &words, AT8), Entry::Foreign, "word {i}");
        }
        // A far jump starting inside them.
        let mut words = stock_words(&CHOICE);
        words[2..6].copy_from_slice(&far(HANDLER));
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Foreign);
        // A branch over the first word, then something else changed.
        let mut words = stock_words(&CHOICE);
        words[0] = b(AT8, HANDLER);
        words[2] ^= 0x10;
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Foreign);
    }

    #[test]
    fn other_hooks_past_the_replaced_words_are_accepted() {
        let mut words = stock_words(&CHOICE);
        words[6..10].copy_from_slice(&far(HANDLER));
        words[11] = b(AT8 + 44, HANDLER);
        words[14..16].copy_from_slice(&FAR_JUMP); // its target is past the window
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Stock);
        words[..4].copy_from_slice(&far(HANDLER));
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Chain(HANDLER));
        // Anything that is not a hook is not (word 13 is an `orr`).
        words[13] ^= 0x10;
        assert_eq!(entry(&CHOICE, &words, AT8), Entry::Foreign);
    }

    #[test]
    fn foreign_flag_parse() {
        assert!(!parse_foreign(b""));
        assert!(parse_foreign(b"force=none\n foreign = 1 \n"));
        assert!(!parse_foreign(b"foreign=1\nforeign=0\n"));
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
        assert_eq!(plan(POPUP_ORDINARY, true, Body::Elite, false), Plan::Keep);
        assert_eq!(
            plan(0x58ff_ff1b_4bcc_ae33, true, Body::Elite, false),
            Plan::Keep
        );
        assert_eq!(
            plan(POPUP_ENTERED, true, Body::Elite, false),
            Plan::Reparse {
                change: Change::Entered,
                keep_body: true
            }
        );
        assert_eq!(
            plan(POPUP_LEFT, true, Body::Rematch, false),
            Plan::Reparse {
                change: Change::Left,
                keep_body: false
            }
        );
        assert_eq!(plan(POPUP_LEFT, false, Body::Elite, false), Plan::Keep);
        assert_eq!(
            plan(POPUP_ENTERED, true, Body::Both, false),
            Plan::Reparse {
                change: Change::Entered,
                keep_body: true
            }
        );
    }

    #[test]
    fn notice_shows_once_then_plain_rematch() {
        for body in [Body::Both, Body::Elite] {
            for record in [POPUP_ENTERED, POPUP_LEFT] {
                let change = Change::of(record).unwrap();
                assert_eq!(
                    plan(record, true, body, true),
                    Plan::Reparse {
                        change,
                        keep_body: false
                    },
                    "later popups keep the rematch without the notice"
                );
            }
        }
        assert_eq!(plan(POPUP_ORDINARY, true, Body::Both, true), Plan::Keep);
        assert_eq!(plan(POPUP_ENTERED, false, Body::Both, true), Plan::Keep);
    }

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    #[test]
    fn combine_joins_notice_and_question() {
        let mut out = [0xffffu16; COMBINED_LEN];
        let len = combine(
            &utf16("You can now participate\nin Elite Smash."),
            &utf16("Rematch?"),
            &mut out,
        );
        let text = String::from_utf16(&out[..len]).unwrap();
        assert_eq!(text, "You can now participate\nin Elite Smash.\nRematch?");
        assert_eq!(out[len], 0);
    }

    #[test]
    fn combine_cuts_to_fit_and_keeps_the_nul() {
        let mut out = [0xffffu16; 8];
        let len = combine(&utf16("Notice"), &utf16("Rematch?"), &mut out);
        assert_eq!(String::from_utf16(&out[..len]).unwrap(), "Notice\n");
        assert_eq!((len, out[len]), (7, 0));
        let len = combine(&utf16("A very long notice"), &utf16("Q"), &mut out);
        assert_eq!(String::from_utf16(&out[..len]).unwrap(), "A very ");
        assert_eq!(combine(&utf16("x"), &utf16("y"), &mut []), 0);
        let mut one = [9u16; 1];
        assert_eq!(
            (combine(&utf16("x"), &utf16("y"), &mut one), one[0]),
            (0, 0)
        );
        let len = combine(&utf16("Notice"), &utf16(""), &mut out);
        assert_eq!(String::from_utf16(&out[..len]).unwrap(), "Notice");
    }

    #[test]
    fn labels_match_with_their_nul() {
        let matches = |read: &[u8], label: &[u8]| {
            label_matches(
                |i| {
                    assert!(i < read.len(), "read past the label");
                    read[i]
                },
                label,
            )
        };
        assert!(matches(b"pop_online_vip_start\0junk", LABEL_ENTERED));
        assert!(!matches(b"pop_online_vip_start_x\0", LABEL_ENTERED));
        assert!(!matches(b"pop\0", LABEL_ENTERED));
        assert!(!matches(b"pop_online_vip_end\0", LABEL_ENTERED));
        assert!(matches(b"pop_online_vip_end\0", LABEL_LEFT));
        assert!(matches(b"pop_result_battle_again\0", LABEL_REMATCH));
        for change in [Change::Entered, Change::Left] {
            assert_eq!(Change::from_code(change.code()), Some(change));
            assert_eq!(change.label().last(), Some(&0));
        }
        assert_eq!(Change::from_code(0), None);
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
    fn threshold_parse() {
        assert_eq!(parse_threshold(b""), None);
        assert_eq!(parse_threshold(b"threshold=9000000\n"), Some(9_000_000));
        assert_eq!(parse_threshold(b"threshold = 0"), Some(0));
        assert_eq!(parse_threshold(b"retain=0\nforce=left"), None);
        assert_eq!(parse_threshold(b"threshold= 12 \r\n"), Some(12));
        assert_eq!(parse_threshold(b"threshold=5\nthreshold=native"), None);
        assert_eq!(parse_threshold(b"threshold=5\nthreshold=junk"), Some(5));
    }

    #[test]
    fn diag_guards_start_with_the_hooked_instruction() {
        assert_eq!(CLASSIFY.guard[..4], [0x1f, 0x00, 0x13, 0x6b]); // cmp w0, w19
        assert_eq!(VIP_ENABLED.guard[..4], [0x1f, 0x05, 0x00, 0x71]); // cmp w8, #1
        assert_eq!(SEARCH_DELAY.guard[..4], [0x20, 0x8f, 0x00, 0xb9]); // str w0, [x25, #0x8c]
    }

    #[test]
    fn settings_round_trip_through_bits() {
        for force in [None, Some(Change::Entered), Some(Change::Left)] {
            for retain in [false, true] {
                for body in [Body::Both, Body::Elite, Body::Rematch] {
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
