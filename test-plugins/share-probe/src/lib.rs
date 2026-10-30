//! Eden test plugin for inline-hook sharing v1: it wants `.147fb64`, the same
//! instruction elite-rematch hooks (the parsed post-match popup). Whichever plugin's export
//! `ssbu_inline_site_147fb64_v1` resolves first owns the hook; the other
//! registers its callback. This one counts its callback runs and logs the
//! count, so a run shows both plugins working from one hook.
#![cfg(target_os = "switch")]

#[path = "../../../src/share.rs"]
#[allow(dead_code)]
mod share;

use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const SITE: usize = 0x147fb64;
/// The first 16 bytes of `.147fb64` (elite-rematch's full guard is 64).
const GUARD: [u8; 16] = [
    0x60, 0x86, 0x40, 0xa9, 0x12, 0xc8, 0x79, 0x94, 0xf3, 0x07, 0x40, 0xf9, 0xf3, 0xf6, 0xff, 0xb5,
];

static HITS: AtomicU64 = AtomicU64::new(0);
static SHARED: share::Callbacks = share::Callbacks::new();
static SITE_SHARE: share::SiteShareV1 = share::SiteShareV1 {
    abi_version: share::ABI_VERSION,
    offset: SITE as u64,
    register,
};

#[no_mangle]
pub extern "C" fn ssbu_inline_site_147fb64_v1() -> *const share::SiteShareV1 {
    &SITE_SHARE
}

/// Set once this plugin's hook is on the site (its bytes no longer match).
static HOOKED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn register(callback: usize) -> bool {
    (HOOKED.load(Ordering::Acquire) || guard_matches()) && SHARED.push(callback)
}

unsafe extern "C" fn probe_callback(_ctx: *mut InlineCtx) {
    HITS.fetch_add(1, Ordering::Relaxed);
}

#[skyline::hook(offset = SITE, inline)]
unsafe fn site(ctx: &mut InlineCtx) {
    probe_callback(ctx);
    SHARED.for_each(|callback| {
        let callback: unsafe extern "C" fn(*mut InlineCtx) = core::mem::transmute(callback);
        callback(ctx);
    });
}

fn guard_matches() -> bool {
    unsafe {
        let address = getRegionAddress(Region::Text) as usize + SITE;
        core::slice::from_raw_parts(address as *const u8, GUARD.len()) == GUARD
    }
}

struct Line {
    buf: [u8; 192],
    len: usize,
}

impl core::fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let take = s.len().min(self.buf.len() - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

fn append(args: core::fmt::Arguments) {
    let mut line = Line {
        buf: [0; 192],
        len: 0,
    };
    let _ = line.write_fmt(args);
    let _ = line.write_char('\n');
    unsafe {
        nn::fs::CreateDirectory(c"sd:/ultimate".as_ptr().cast());
        nn::fs::CreateDirectory(c"sd:/ultimate/share-probe".as_ptr().cast());
        let path = c"sd:/ultimate/share-probe/events.log".as_ptr().cast();
        nn::fs::CreateFile(path, 0);
        let mut handle = nn::fs::FileHandle { handle: 0 };
        if nn::fs::OpenFile(&mut handle, path, 2 | 4) != 0 {
            return;
        }
        let mut offset = 0;
        if nn::fs::GetFileSize(&mut offset, handle) == 0 {
            let option = nn::fs::WriteOption { flags: 0 };
            nn::fs::WriteFile(handle, offset, line.buf.as_ptr(), line.len as u64, &option);
            nn::fs::FlushFile(handle);
        }
        nn::fs::CloseFile(handle);
    }
}

extern "C" fn writer(_: *mut skyline::libc::c_void) {
    let mut logged = 0;
    loop {
        unsafe {
            nn::os::SleepThread(nn::TimeSpan {
                nanoseconds: 500_000_000,
            })
        };
        let hits = HITS.load(Ordering::Relaxed);
        if hits != logged {
            logged = hits;
            append(format_args!("probe hits={hits}"));
        }
    }
}

#[repr(C, align(4096))]
struct Stack(core::cell::UnsafeCell<[u8; 16384]>);
#[repr(C, align(16))]
struct Thread(core::cell::UnsafeCell<[u8; 0x1c0]>);
unsafe impl Sync for Stack {}
unsafe impl Sync for Thread {}
static STACK: Stack = Stack(core::cell::UnsafeCell::new([0; 16384]));
static THREAD: Thread = Thread(core::cell::UnsafeCell::new([0; 0x1c0]));

#[skyline::main(name = "share-probe")]
pub fn main() {
    // The owner first: when it loaded earlier its hook is already on the
    // site, so only an owner checks the site's bytes.
    let mut export = 0usize;
    unsafe { nn::ro::LookupSymbol(&mut export, b"ssbu_inline_site_147fb64_v1\0".as_ptr()) };
    let table = if export == 0 {
        0
    } else {
        let export: extern "C" fn() -> *const share::SiteShareV1 =
            unsafe { core::mem::transmute(export) };
        export() as usize
    };
    match share::role(table, &SITE_SHARE as *const _ as usize) {
        share::Role::Owner => {
            if !guard_matches() {
                append(format_args!("probe result=guard_mismatch role=owner"));
                return;
            }
            skyline::install_hooks!(site);
            HOOKED.store(true, Ordering::Release);
            append(format_args!(
                "probe result=installed role=owner guests={}",
                SHARED.len()
            ));
        }
        share::Role::Guest(owner) => {
            let api = unsafe { &*(owner as *const share::SiteShareV1) };
            let ok = share::compatible(api, SITE)
                && unsafe { (api.register)(probe_callback as *const () as usize) };
            append(format_args!(
                "probe result=registered role=guest owner={owner:#x} ok={ok}"
            ));
        }
    }
    unsafe {
        let thread = THREAD.0.get().cast::<nn::os::ThreadType>();
        let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
        if nn::os::CreateThread(
            thread,
            writer,
            core::ptr::null_mut(),
            STACK.0.get().cast(),
            16384,
            priority,
        ) == 0
        {
            nn::os::StartThread(thread);
        }
    }
}
