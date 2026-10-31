use std::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

type RequestCallback = extern "C" fn(u64);

struct State {
    sent: u64,
    acknowledged: u64,
    elapsed_ns: u64,
}

struct Bridge {
    callback: RequestCallback,
    total: u64,
    window: u64,
    state: Arc<(Mutex<State>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

#[unsafe(no_mangle)]
pub extern "C" fn dust_bridge_new(
    callback: RequestCallback,
    total: u64,
    window: u64,
) -> *mut c_void {
    if total == 0 || window == 0 {
        return std::ptr::null_mut();
    }
    let bridge = Bridge {
        callback,
        total,
        window,
        state: Arc::new((
            Mutex::new(State {
                sent: 0,
                acknowledged: 0,
                elapsed_ns: 0,
            }),
            Condvar::new(),
        )),
        thread: None,
    };
    Box::into_raw(Box::new(bridge)).cast()
}

#[unsafe(no_mangle)]
/// Starts the probe's native producer thread.
///
/// # Safety
///
/// `raw` must be a live pointer returned by [`dust_bridge_new`], and no other
/// thread may call this function for the same bridge concurrently.
pub unsafe extern "C" fn dust_bridge_start(raw: *mut c_void) -> bool {
    let Some(bridge) = (unsafe { raw.cast::<Bridge>().as_mut() }) else {
        return false;
    };
    if bridge.thread.is_some() {
        return false;
    }

    let callback = bridge.callback;
    let total = bridge.total;
    let window = bridge.window;
    let state = Arc::clone(&bridge.state);
    bridge.thread = Some(thread::spawn(move || {
        let started = Instant::now();
        for id in 0..total {
            let (lock, ready) = &*state;
            let mut current = lock.lock().unwrap_or_else(|error| error.into_inner());
            while current.sent - current.acknowledged >= window {
                current = ready
                    .wait(current)
                    .unwrap_or_else(|error| error.into_inner());
            }
            current.sent += 1;
            drop(current);
            callback(id);
        }

        let (lock, ready) = &*state;
        let mut current = lock.lock().unwrap_or_else(|error| error.into_inner());
        while current.acknowledged < total {
            current = ready
                .wait(current)
                .unwrap_or_else(|error| error.into_inner());
        }
        current.elapsed_ns = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        ready.notify_all();
    }));
    true
}

#[unsafe(no_mangle)]
/// Acknowledges one callback delivered to Dart.
///
/// # Safety
///
/// `raw` must remain a live pointer returned by [`dust_bridge_new`].
pub unsafe extern "C" fn dust_bridge_ack(raw: *mut c_void, _id: u64) {
    let Some(bridge) = (unsafe { raw.cast::<Bridge>().as_ref() }) else {
        return;
    };
    let (lock, ready) = &*bridge.state;
    let mut current = lock.lock().unwrap_or_else(|error| error.into_inner());
    current.acknowledged += 1;
    ready.notify_all();
}

#[unsafe(no_mangle)]
/// Waits for completion and returns native wall-clock nanoseconds.
///
/// # Safety
///
/// `raw` must remain a live pointer returned by [`dust_bridge_new`].
pub unsafe extern "C" fn dust_bridge_elapsed_ns(raw: *mut c_void) -> u64 {
    let Some(bridge) = (unsafe { raw.cast::<Bridge>().as_ref() }) else {
        return 0;
    };
    let (lock, ready) = &*bridge.state;
    let mut current = lock.lock().unwrap_or_else(|error| error.into_inner());
    while current.elapsed_ns == 0 {
        current = ready
            .wait(current)
            .unwrap_or_else(|error| error.into_inner());
    }
    current.elapsed_ns
}

#[unsafe(no_mangle)]
/// Joins the producer thread and releases the bridge.
///
/// # Safety
///
/// `raw` must be null or a live pointer returned by [`dust_bridge_new`] that
/// has not already been freed. No other function may use it after this call.
pub unsafe extern "C" fn dust_bridge_free(raw: *mut c_void) {
    if raw.is_null() {
        return;
    }
    let mut bridge = unsafe { Box::from_raw(raw.cast::<Bridge>()) };
    if let Some(thread) = bridge.thread.take() {
        let _ = thread.join();
    }
}
