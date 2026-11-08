//! A tool as a wasm module for the renderer's wasm sessions (`SPEC.md`,
//! section 4): `receive` runs the app on the lines it is given and passes
//! what the app wrote to the renderer's `hyperb.send`. An app that ticks
//! asks the renderer to `wake` it, again after every tick, for as long as
//! it wants ticks.

/// Selects the single app this module hosts. Exactly one app feature must be
/// enabled: the module exports one `receive` and owns one `Server<App>`, so
/// `just wasm-apps` builds each app separately.
macro_rules! select_app {
    ($($feature:literal => $app:ty),* $(,)?) => {
        $(
            #[cfg(feature = $feature)]
            type App = $app;
        )*

        const _: () = {
            let mut count = 0;
            $( if cfg!(feature = $feature) { count += 1; } )*
            assert!(
                count == 1,
                "build the wasm module with exactly one app feature, \
                 e.g. `--no-default-features --features worldclock`"
            );
        };
    };
}

select_app! {
    "billsplit" => crate::billsplit::BillSplit,
    "finance" => crate::finance::Finance,
    "qr" => crate::qr::Qr,
    "textutils" => crate::textutils::TextUtils,
    "wordle" => crate::wordle::Wordle,
    "worldclock" => crate::worldclock::WorldClock,
}

use hypereffect::server::Server;
use std::cell::{Cell, RefCell};

#[link(wasm_import_module = "hyperb")]
unsafe extern "C" {
    fn send(ptr: *const u8, len: u32);
    fn now() -> f64;
    fn wake_in(ms: i32);
}

thread_local! {
    static SERVER: RefCell<Server<App>> = RefCell::new(Server::new());
    /// The tick interval of the pending wake-up, if one is.
    static WAKING: Cell<Option<u32>> = const { Cell::new(None) };
}

/// A buffer for the renderer to write a line into; `receive` takes it back.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    Box::into_raw(vec![0u8; len as usize].into_boxed_slice()) as *mut u8
}

#[unsafe(no_mangle)]
pub extern "C" fn receive(ptr: *mut u8, len: u32) {
    // SAFETY: `ptr` came from `alloc(len)` and is received once.
    let input = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len as usize)) };
    SERVER.with_borrow_mut(|s| {
        s.receive(&input);
        flush(s);
    });
}

/// The wake-up `flush` asked for.
#[unsafe(no_mangle)]
pub extern "C" fn wake() {
    WAKING.set(None);
    SERVER.with_borrow_mut(|s| {
        // SAFETY: an import without arguments.
        s.tick(unsafe { now() });
        flush(s);
    });
}

/// Sends what the app wrote, and asks for the wake-up of its next tick,
/// or cancels the pending one if it no longer ticks at that interval.
fn flush(s: &mut Server<App>) {
    let out = s.output().take();
    if !out.is_empty() {
        // SAFETY: the buffer is valid for the call.
        unsafe { send(out.as_ptr(), out.len() as u32) };
    }
    let want = s.tick_interval();
    if want != WAKING.get() {
        let ms = want.map_or(-1, |ms| ms.min(i32::MAX as u32) as i32);
        // SAFETY: an import taking a number.
        unsafe { wake_in(ms) };
        WAKING.set(want);
    }
}
