//! Switch half: version check, guarded inline hooks on the post-match popup,
//! an event ring and a writer thread that appends `events.log`.
//!
//! Hooks run on the game's UI thread and only format a line into a static
//! ring; the writer thread does the file I/O. No heap (`heap_allocations` in
//! the install line counts any).

use crate::model::{self, descriptor, Body, Change, Line, Plan, Settings, Site};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{fence, AtomicU32, AtomicU64, Ordering};

const DATA_DIR: &str = "sd:/ultimate/elite-rematch";
const EVENT_RING: usize = 16;
const EVENT_LEN: usize = 448;
const WRITER_STACK: usize = 32 * 1024;
#[cfg(feature = "diag")]
const OPEN_READ: i32 = 1;
const OPEN_WRITE_APPEND: i32 = 2 | 4;

/// Counts calls into the heap. Nothing in the plugin is meant to make one.
struct CountingAllocator;
static HEAP_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Shared cell whose accesses are ordered by the surrounding atomics.
struct Owned<T>(UnsafeCell<T>);
// SAFETY: see each use; writers publish through a sequence number or before
// the hooks are installed.
unsafe impl<T> Sync for Owned<T> {}

/// One formatted line; `seq` is the event index + 1 once it is complete.
struct EventSlot {
    seq: AtomicU64,
    text: Owned<Line<EVENT_LEN>>,
}

static EVENTS: [EventSlot; EVENT_RING] = [const {
    EventSlot {
        seq: AtomicU64::new(0),
        text: Owned(UnsafeCell::new(Line::new())),
    }
}; EVENT_RING];
static EVENT_HEAD: AtomicU64 = AtomicU64::new(0);

static TEXT_BASE: AtomicU64 = AtomicU64::new(0);
/// The popup on screen: 0 ordinary, 1/2 entered/left as the game shows it,
/// 3/4 entered/left with the rematch kept.
static SHOWN: AtomicU32 = AtomicU32::new(0);
/// `Settings::to_bits`; diagnostic builds reload it from `diag.txt`.
static SETTINGS: AtomicU32 = AtomicU32::new(0);
/// `Change::code` of the kept-rematch popup on screen whose notice is shown
/// with the question; 0 otherwise. Set and read on the UI thread.
static COMBINE: AtomicU32 = AtomicU32::new(0);
/// The combined text handed to the game; static, so it outlives the lookup.
static COMBINED: Owned<[u16; model::COMBINED_LEN]> =
    Owned(UnsafeCell::new([0; model::COMBINED_LEN]));
#[cfg(feature = "diag")]
static COMBINE_LOGGED: AtomicU32 = AtomicU32::new(0);

/// The writer thread's object and stack, static so no heap is involved.
#[repr(C, align(4096))]
struct ThreadStack(UnsafeCell<[u8; WRITER_STACK]>);
#[repr(C, align(16))]
struct ThreadObject(UnsafeCell<[u8; 0x1c0]>);
// SAFETY: handed to nn::os once, at startup, and never touched again.
unsafe impl Sync for ThreadStack {}
unsafe impl Sync for ThreadObject {}
static WRITER_STACK_MEMORY: ThreadStack = ThreadStack(UnsafeCell::new([0; WRITER_STACK]));
static WRITER_THREAD: ThreadObject = ThreadObject(UnsafeCell::new([0; 0x1c0]));

fn tick() -> u64 {
    unsafe { nn::os::GetSystemTick() as u64 }
}

unsafe fn read_u32(address: u64) -> u32 {
    (address as *const u32).read_unaligned()
}

unsafe fn read_u64(address: u64) -> u64 {
    (address as *const u64).read_unaligned()
}

fn settings() -> Settings {
    Settings::from_bits(SETTINGS.load(Ordering::Acquire))
}

fn push_event(args: core::fmt::Arguments) {
    let index = EVENT_HEAD.fetch_add(1, Ordering::AcqRel);
    let slot = &EVENTS[index as usize % EVENT_RING];
    slot.seq.store(0, Ordering::Release);
    // SAFETY: this index owns the slot until it publishes `seq`; the writer
    // re-checks `seq` after copying and drops a line overwritten meanwhile.
    let text = unsafe { &mut *slot.text.0.get() };
    text.clear();
    let _ = write!(text, "{} ", tick());
    let _ = text.write_fmt(args);
    let _ = text.write_char('\n');
    slot.seq.store(index + 1, Ordering::Release);
}

fn shown_name(shown: u32) -> &'static str {
    match shown {
        1 => "entered_native",
        2 => "left_native",
        3 => "entered_kept",
        4 => "left_kept",
        _ => "ordinary",
    }
}

/// Eden-only: replaces the ordinary record with the forced Elite Smash one,
/// as if the fighter had just entered or left Elite Smash.
#[cfg(feature = "diag")]
#[skyline::hook(offset = model::SELECT_OFFSET, inline)]
unsafe fn popup_select(ctx: &mut InlineCtx) {
    let Some(change) = settings().force else {
        return;
    };
    let record = ctx.registers[20].x();
    if record == model::POPUP_ORDINARY {
        ctx.registers[20].set_x(change.record());
        push_event(format_args!(
            "diag force={} record={record:#x}",
            change.name()
        ));
    }
}

#[cfg(feature = "diag")]
unsafe fn log_descriptor(stage: &str, d: u64) {
    use descriptor::*;
    let text = |offset: usize| {
        model::label(core::slice::from_raw_parts(
            (d as usize + offset) as *const u8,
            LABEL_LEN,
        ))
    };
    push_event(format_args!(
        "diag {stage} record={:#x} style={} default={} body_hash={:#x} body={} b1={} b2={} b3={} args={:#x}:{:#x}",
        read_u64(d + RECORD as u64),
        read_u32(d + STYLE as u64),
        read_u32(d + DEFAULT_BUTTON as u64),
        read_u32(d + BODY as u64),
        text(BODY_LABEL),
        text(BUTTON_LABELS[0]),
        text(BUTTON_LABELS[1]),
        text(BUTTON_LABELS[2]),
        read_u64(d + 0x140),
        read_u64(d + 0x148),
    ));
}

/// The parsed popup, before it is shown. For an Elite Smash change, parses
/// the ordinary (rematch) record into the same descriptor with the game's own
/// parser, then puts the Elite Smash text back.
#[skyline::hook(offset = model::PARSED_OFFSET, inline)]
unsafe fn popup_parsed(ctx: &mut InlineCtx) {
    let record = ctx.registers[20].x();
    let descriptor = read_u64(ctx.registers[19].x() + 0x10);
    #[cfg(feature = "diag")]
    log_descriptor("parsed", descriptor);
    let settings = settings();
    match model::plan(record, settings.retain, settings.body) {
        Plan::Keep => {
            COMBINE.store(0, Ordering::Release);
            let change = Change::of(record);
            SHOWN.store(
                match change {
                    Some(Change::Entered) => 1,
                    Some(Change::Left) => 2,
                    None => 0,
                },
                Ordering::Release,
            );
            push_event(format_args!(
                "popup record={record:#x} change={} rematch=native",
                change.map_or("none", Change::name)
            ));
        }
        Plan::Reparse { change, keep_body } => {
            let body = (descriptor as usize + descriptor::BODY) as *mut u8;
            let mut saved = [0u8; descriptor::BODY_END - descriptor::BODY];
            if keep_body {
                core::ptr::copy_nonoverlapping(body, saved.as_mut_ptr(), saved.len());
            }
            let parse = core::mem::transmute::<*const (), extern "C" fn(u64, u64)>(
                (TEXT_BASE.load(Ordering::Relaxed) as usize + model::PARSE_POPUP_OFFSET)
                    as *const (),
            );
            parse(descriptor, model::POPUP_ORDINARY);
            if keep_body {
                core::ptr::copy_nonoverlapping(saved.as_ptr(), body, saved.len());
            }
            // The body is set from its label while the popup is shown, through
            // the message lookup below.
            COMBINE.store(
                if settings.body == Body::Both {
                    change.code()
                } else {
                    0
                },
                Ordering::Release,
            );
            #[cfg(feature = "diag")]
            COMBINE_LOGGED.store(0, Ordering::Release);
            SHOWN.store(
                match change {
                    Change::Entered => 3,
                    Change::Left => 4,
                },
                Ordering::Release,
            );
            push_event(format_args!(
                "popup record={record:#x} change={} rematch=kept body={}",
                change.name(),
                settings.body.name()
            ));
            #[cfg(feature = "diag")]
            log_descriptor("reparsed", descriptor);
        }
    }
}

/// The popup's completion (read-only): which button was chosen.
#[skyline::hook(offset = model::CHOICE_OFFSET, inline)]
unsafe fn popup_choice(ctx: &mut InlineCtx) {
    let button = read_u32(ctx.registers[8].x() + 0x118);
    let shown = SHOWN.swap(0, Ordering::AcqRel);
    COMBINE.store(0, Ordering::Release);
    push_event(format_args!(
        "choice button={button} rematch={} popup={}",
        u8::from(button == model::REMATCH_BUTTON),
        shown_name(shown)
    ));
}

/// A NUL-terminated UTF-16 text of at most `max` units (without the NUL).
unsafe fn utf16_at<'a>(text: *const u16, max: usize) -> &'a [u16] {
    if text.is_null() {
        return &[];
    }
    let mut len = 0;
    while len < max && *text.add(len) != 0 {
        len += 1;
    }
    core::slice::from_raw_parts(text, len)
}

/// Every message lookup by label. While a kept-rematch popup is up, its
/// Elite Smash notice is answered with the notice, a line break and the
/// game's own "Rematch?", both in the console's language.
#[skyline::hook(offset = model::MESSAGE_LOOKUP_OFFSET)]
unsafe fn message_lookup(manager: u64, label: *const u8) -> *const u16 {
    let text = call_original!(manager, label);
    let Some(change) = Change::from_code(COMBINE.load(Ordering::Acquire)) else {
        return text;
    };
    if label.is_null() || !model::label_matches(|i| *label.add(i), change.label()) {
        return text;
    }
    let question = call_original!(manager, model::LABEL_REMATCH.as_ptr());
    // SAFETY: only the UI thread looks texts up while COMBINE is set.
    let out = &mut *COMBINED.0.get();
    let _len = model::combine(
        utf16_at(text, model::COMBINED_LEN),
        utf16_at(question, model::COMBINED_LEN),
        out,
    );
    #[cfg(feature = "diag")]
    if COMBINE_LOGGED.swap(1, Ordering::AcqRel) == 0 {
        let mut ascii = Line::<160>::new();
        for unit in &out[.._len] {
            let c = char::from_u32(u32::from(*unit)).filter(char::is_ascii_graphic);
            let _ = ascii.write_char(match (*unit, c) {
                (0x0a, _) => '|',
                (0x20, _) => ' ',
                (_, Some(c)) => c,
                _ => '?',
            });
        }
        push_event(format_args!(
            "diag combined units={_len} text={}",
            core::str::from_utf8(ascii.as_bytes()).unwrap_or("")
        ));
    }
    out.as_ptr()
}

type Path = Line<96>;

fn path(args: core::fmt::Arguments) -> Path {
    let mut path = Path::new();
    let _ = path.write_fmt(args);
    path
}

/// An `nn::fs` file opened to append; closed on drop. A closed file is on
/// the card at once, while Eden keeps an open file's writes until the host
/// file is closed (its `FlushFile` does not reach the host).
struct NxFile {
    handle: nn::fs::FileHandle,
    offset: i64,
}

impl NxFile {
    fn append(path: &mut Path) -> Option<Self> {
        let path = path.with_nul()?.as_ptr();
        unsafe {
            // Fails harmlessly when the file exists.
            nn::fs::CreateFile(path, 0);
            let mut handle = nn::fs::FileHandle { handle: 0 };
            if nn::fs::OpenFile(&mut handle, path, OPEN_WRITE_APPEND) != 0 {
                return None;
            }
            let mut offset = 0;
            if nn::fs::GetFileSize(&mut offset, handle) != 0 {
                nn::fs::CloseFile(handle);
                return None;
            }
            Some(Self { handle, offset })
        }
    }

    fn write(&mut self, bytes: &[u8]) -> bool {
        let option = nn::fs::WriteOption { flags: 0 };
        let written = unsafe {
            nn::fs::WriteFile(
                self.handle,
                self.offset,
                bytes.as_ptr(),
                bytes.len() as u64,
                &option,
            )
        } == 0;
        if written {
            self.offset += bytes.len() as i64;
        }
        written
    }
}

impl Drop for NxFile {
    fn drop(&mut self) {
        unsafe {
            nn::fs::FlushFile(self.handle);
            nn::fs::CloseFile(self.handle);
        }
    }
}

fn append_log(bytes: &[u8]) {
    if let Some(mut log) = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log"))) {
        log.write(bytes);
    }
}

/// `diag.txt` into `buf`; the bytes read (0 when it is missing).
#[cfg(feature = "diag")]
fn read_diag(buf: &mut [u8; 256]) -> usize {
    let mut file = path(format_args!("{DATA_DIR}/diag.txt"));
    let Some(file) = file.with_nul() else {
        return 0;
    };
    let mut read = 0u64;
    unsafe {
        let mut handle = nn::fs::FileHandle { handle: 0 };
        if nn::fs::OpenFile(&mut handle, file.as_ptr(), OPEN_READ) != 0 {
            return 0;
        }
        let ok = nn::fs::ReadFile1(&mut read, handle, 0, buf.as_mut_ptr(), buf.len() as u64) == 0;
        nn::fs::CloseFile(handle);
        if !ok {
            return 0;
        }
    }
    (read as usize).min(buf.len())
}

/// Settings and threshold override from `diag.txt`.
#[cfg(feature = "diag")]
fn read_settings() -> (Settings, Option<u32>) {
    let mut buf = [0u8; 256];
    let len = read_diag(&mut buf);
    (
        Settings::parse(&buf[..len]),
        model::parse_threshold(&buf[..len]),
    )
}

/// 0 = the game's threshold, else `1 << 32 | value`.
#[cfg(feature = "diag")]
static THRESHOLD: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "diag")]
fn threshold_word(threshold: Option<u32>) -> u64 {
    threshold.map_or(0, |t| (1 << 32) | u64::from(t))
}

/// Last classifier inputs logged, so a per-frame caller logs each state once.
#[cfg(feature = "diag")]
static CLASSIFY_SEEN: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
#[cfg(feature = "diag")]
static CLASSIFY_NEXT: AtomicU32 = AtomicU32::new(0);

/// Eden-only: the Elite Smash compare (threshold w0 vs rating w19, fighter
/// x20). Logs each new input once and applies the `threshold=` override.
#[cfg(feature = "diag")]
#[skyline::hook(offset = model::CLASSIFY_OFFSET, inline)]
unsafe fn classify(ctx: &mut InlineCtx) {
    let native = ctx.registers[0].w();
    let rating = ctx.registers[19].w();
    let fighter = ctx.registers[20].x();
    let word = THRESHOLD.load(Ordering::Acquire);
    let used = if word >> 32 != 0 { word as u32 } else { native };
    if used != native {
        ctx.registers[0].set_w(used);
    }
    let key =
        (fighter & 0xff_ffff_ffff) ^ (u64::from(rating) << 24) ^ u64::from(used).rotate_left(48);
    if CLASSIFY_SEEN
        .iter()
        .any(|seen| seen.load(Ordering::Relaxed) == key)
    {
        return;
    }
    let slot = CLASSIFY_NEXT.fetch_add(1, Ordering::Relaxed) as usize % CLASSIFY_SEEN.len();
    CLASSIFY_SEEN[slot].store(key, Ordering::Relaxed);
    push_event(format_args!(
        "diag classify fighter={fighter:#x} rating={rating} threshold={native} used={used} elite={}",
        u8::from(rating >= used)
    ));
}

/// Eden-only: logs the Elite Smash search delay (parameter 1199, seconds) and
/// the search mode each time a search is set up. Read-only.
#[cfg(feature = "diag")]
#[skyline::hook(offset = model::SEARCH_DELAY_OFFSET, inline)]
unsafe fn search_delay(ctx: &mut InlineCtx) {
    let delay = ctx.registers[0].w();
    let mode = read_u32(ctx.registers[25].x() - 4);
    push_event(format_args!(
        "diag search_delay seconds={delay} mode={mode}"
    ));
}

/// Eden-only: logs the "Elite Smash enabled" parameter when it changes.
#[cfg(feature = "diag")]
#[skyline::hook(offset = model::VIP_ENABLED_OFFSET, inline)]
unsafe fn vip_enabled(ctx: &mut InlineCtx) {
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    let value = ctx.registers[8].w();
    if LAST.swap(value, Ordering::Relaxed) != value {
        push_event(format_args!("diag vip_enabled param9010={value}"));
    }
}

/// Copies every published event after `tail` into `events.log`.
fn drain(tail: &mut u64) {
    let head = EVENT_HEAD.load(Ordering::Acquire);
    if head == *tail {
        return;
    }
    if head - *tail > EVENT_RING as u64 {
        *tail = head - EVENT_RING as u64;
    }
    let Some(mut file) = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log"))) else {
        return;
    };
    while *tail < head {
        let slot = &EVENTS[*tail as usize % EVENT_RING];
        let seq = slot.seq.load(Ordering::Acquire);
        if seq == 0 || seq < *tail + 1 {
            break; // still being written
        }
        let mut copy = Line::<EVENT_LEN>::new();
        // SAFETY: read under the sequence check below.
        let text = unsafe { &*slot.text.0.get() };
        let _ = copy.write_str(core::str::from_utf8(text.as_bytes()).unwrap_or(""));
        fence(Ordering::Acquire);
        if seq == *tail + 1 && slot.seq.load(Ordering::Relaxed) == seq {
            file.write(copy.as_bytes());
        }
        *tail += 1;
    }
}

/// Diagnostics: picks up edits of `diag.txt` between matches.
#[cfg(feature = "diag")]
fn reload_settings() {
    let (loaded, threshold) = read_settings();
    let bits = loaded.to_bits();
    let word = threshold_word(threshold);
    let changed = SETTINGS.swap(bits, Ordering::AcqRel) != bits;
    if THRESHOLD.swap(word, Ordering::AcqRel) != word || changed {
        push_event(format_args!(
            "diag settings force={} retain={} body={} threshold={}",
            loaded.force.map_or("none", Change::name),
            u8::from(loaded.retain),
            loaded.body.name(),
            threshold.map_or(-1, i64::from)
        ));
    }
}

extern "C" fn writer_main(_: *mut skyline::libc::c_void) {
    let mut tail = 0;
    #[cfg(feature = "diag")]
    let mut round = 0u32;
    loop {
        unsafe {
            nn::os::SleepThread(nn::TimeSpan {
                nanoseconds: 200_000_000,
            })
        };
        #[cfg(feature = "diag")]
        {
            round = round.wrapping_add(1);
            if round.is_multiple_of(10) {
                reload_settings();
            }
        }
        drain(&mut tail);
    }
}

fn start_writer() -> bool {
    unsafe {
        let thread = WRITER_THREAD.0.get().cast::<nn::os::ThreadType>();
        let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
        let created = nn::os::CreateThread(
            thread,
            writer_main,
            core::ptr::null_mut(),
            WRITER_STACK_MEMORY.0.get().cast::<u8>(),
            WRITER_STACK,
            priority,
        );
        if created != 0 {
            return false;
        }
        nn::os::SetThreadNamePointer(thread, c"elite-rematch".as_ptr().cast());
        nn::os::StartThread(thread);
        true
    }
}

unsafe fn guard_matches(site: &Site) -> bool {
    let base = getRegionAddress(Region::Text) as usize;
    let end = getRegionAddress(Region::Rodata) as usize;
    let address = base + site.offset;
    address.is_multiple_of(4)
        && address + site.guard.len() <= end
        && core::slice::from_raw_parts(address as *const u8, site.guard.len()) == site.guard
}

fn version_is_supported() -> bool {
    let mut version = nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { nn::oe::GetDisplayVersion(&mut version) };
    let end = version.name.iter().position(|v| *v == 0).unwrap_or(16);
    version.name[..end] == *model::SUPPORTED_VERSION.as_bytes()
}

/// Appends the install line to `events.log` and the Skyline log.
fn report_install(report: &mut Line<256>) {
    let _ = write!(
        report,
        " heap_allocations={}",
        HEAP_ALLOCATIONS.load(Ordering::Relaxed)
    );
    skyline::logging::log(core::str::from_utf8(report.as_bytes()).unwrap_or(""));
    let _ = report.write_char('\n');
    for directory in ["sd:/ultimate", DATA_DIR] {
        if let Some(directory) = path(format_args!("{directory}")).with_nul() {
            unsafe { nn::fs::CreateDirectory(directory.as_ptr()) };
        }
    }
    append_log(report.as_bytes());
}

#[skyline::main(name = "elite-rematch")]
pub fn main() {
    let mut report = Line::<256>::new();
    let _ = write!(
        report,
        "{} plugin=elite-rematch/{}",
        tick(),
        env!("CARGO_PKG_VERSION")
    );
    if !version_is_supported() {
        let _ = write!(report, " result=unsupported_version");
        report_install(&mut report);
        return;
    }
    let sites: &[&Site] = if cfg!(feature = "diag") {
        &[
            &model::SELECT,
            &model::PARSED,
            &model::CHOICE,
            &model::MESSAGE_LOOKUP,
            &model::CLASSIFY,
            &model::VIP_ENABLED,
            &model::SEARCH_DELAY,
        ]
    } else {
        &[&model::PARSED, &model::CHOICE, &model::MESSAGE_LOOKUP]
    };
    for site in sites {
        if !unsafe { guard_matches(site) } {
            let _ = write!(report, " result=guard_mismatch site={}", site.name);
            report_install(&mut report);
            return;
        }
    }
    TEXT_BASE.store(
        unsafe { getRegionAddress(Region::Text) as u64 },
        Ordering::Relaxed,
    );
    SETTINGS.store(Settings::DEFAULT.to_bits(), Ordering::Release);
    #[cfg(feature = "diag")]
    {
        let (loaded, threshold) = read_settings();
        SETTINGS.store(loaded.to_bits(), Ordering::Release);
        THRESHOLD.store(threshold_word(threshold), Ordering::Release);
        let _ = write!(
            report,
            " diag force={} retain={} body={} threshold={}",
            loaded.force.map_or("none", Change::name),
            u8::from(loaded.retain),
            loaded.body.name(),
            threshold.map_or(-1, i64::from)
        );
    }
    skyline::install_hooks!(popup_parsed, popup_choice, message_lookup);
    #[cfg(feature = "diag")]
    skyline::install_hooks!(popup_select, classify, vip_enabled, search_delay);
    let _ = write!(report, " result=installed hooks=");
    for (i, site) in sites.iter().enumerate() {
        let _ = write!(report, "{}{}", if i > 0 { "," } else { "" }, site.name);
    }
    report_install(&mut report);
    if !start_writer() {
        report.clear();
        let _ = write!(report, "{} writer=not_started", tick());
        report_install(&mut report);
    }
}
