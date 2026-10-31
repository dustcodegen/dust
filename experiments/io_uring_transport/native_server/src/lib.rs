mod http;
mod runtime;
mod uring_ops;

use std::ffi::c_void;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};

use io_uring::IoUring;
use runtime::{RequestCallback, ResponseMessage};

struct Server {
    responses: mpsc::Sender<ResponseMessage>,
    wake_fd: Arc<OwnedFd>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: u16,
}

#[unsafe(no_mangle)]
pub extern "C" fn dust_io_uring_server_start(callback: RequestCallback, port: u16) -> *mut c_void {
    let Ok(ring) = IoUring::new(256) else {
        return std::ptr::null_mut();
    };
    let address = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    let Ok(listener) = TcpListener::bind(address) else {
        return std::ptr::null_mut();
    };
    let Ok(local_address) = listener.local_addr() else {
        return std::ptr::null_mut();
    };
    let raw_wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
    if raw_wake_fd < 0 {
        return std::ptr::null_mut();
    }
    let wake_fd = Arc::new(unsafe { OwnedFd::from_raw_fd(raw_wake_fd) });
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let runtime_wake_fd = Arc::clone(&wake_fd);
    let runtime_stop = Arc::clone(&stop);
    let thread = thread::spawn(move || {
        if let Err(error) = runtime::run(
            ring,
            listener,
            runtime_wake_fd,
            receiver,
            runtime_stop,
            callback,
        ) {
            eprintln!("io_uring probe stopped: {error}");
        }
    });
    let server = Server {
        responses: sender,
        wake_fd,
        stop,
        thread: Some(thread),
        port: local_address.port(),
    };
    Box::into_raw(Box::new(server)).cast()
}

#[unsafe(no_mangle)]
/// Returns the bound loopback port.
///
/// # Safety
///
/// `raw` must be a live pointer returned by [`dust_io_uring_server_start`].
pub unsafe extern "C" fn dust_io_uring_server_port(raw: *mut c_void) -> u16 {
    unsafe { raw.cast::<Server>().as_ref() }.map_or(0, |server| server.port)
}

#[unsafe(no_mangle)]
pub extern "C" fn dust_io_uring_response_buffer_new(length: usize) -> *mut u8 {
    if length == 0 {
        return std::ptr::null_mut();
    }
    let mut body = vec![0; length].into_boxed_slice();
    let pointer = body.as_mut_ptr();
    std::mem::forget(body);
    pointer
}

#[unsafe(no_mangle)]
/// Delivers a Dart handler response and consumes `body`.
///
/// # Safety
///
/// `raw` must be a live pointer returned by [`dust_io_uring_server_start`].
/// For a nonzero `length`, `body` must be returned by
/// [`dust_io_uring_response_buffer_new`] with that exact length and not have
/// been consumed previously.
pub unsafe extern "C" fn dust_io_uring_server_respond(
    raw: *mut c_void,
    id: u64,
    status: u16,
    body: *mut u8,
    length: usize,
) -> bool {
    let Some(server) = (unsafe { raw.cast::<Server>().as_ref() }) else {
        return false;
    };
    let body: Box<[u8]> = if length == 0 {
        Box::default()
    } else {
        if body.is_null() {
            return false;
        }
        unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(body, length)) }
    };
    if server
        .responses
        .send(ResponseMessage { id, status, body })
        .is_err()
    {
        return false;
    }
    wake(&server.wake_fd)
}

#[unsafe(no_mangle)]
/// Stops and releases the probe server.
///
/// # Safety
///
/// `raw` must be null or a live pointer returned by
/// [`dust_io_uring_server_start`] that has not already been freed. No other
/// function may use it after this call.
pub unsafe extern "C" fn dust_io_uring_server_free(raw: *mut c_void) {
    if raw.is_null() {
        return;
    }
    let mut server = unsafe { Box::from_raw(raw.cast::<Server>()) };
    server.stop.store(true, Ordering::Release);
    let _ = wake(&server.wake_fd);
    if let Some(thread) = server.thread.take() {
        let _ = thread.join();
    }
}

fn wake(fd: &OwnedFd) -> bool {
    let value = 1u64;
    let written = unsafe {
        libc::write(
            fd.as_raw_fd(),
            (&raw const value).cast(),
            std::mem::size_of::<u64>(),
        )
    };
    written == std::mem::size_of::<u64>() as isize
}
