use std::io;

use io_uring::{IoUring, opcode, squeue, types};

use crate::runtime::{ACCEPT, WAKE};

pub(crate) fn submit_accept(ring: &mut IoUring, fd: i32) -> io::Result<()> {
    push(
        ring,
        opcode::Accept::new(types::Fd(fd), std::ptr::null_mut(), std::ptr::null_mut())
            .build()
            .user_data(ACCEPT),
    )
}

pub(crate) fn submit_wake_read(
    ring: &mut IoUring,
    fd: i32,
    value: &mut Box<u64>,
) -> io::Result<()> {
    push(
        ring,
        opcode::Read::new(types::Fd(fd), value.as_mut() as *mut u64 as *mut u8, 8)
            .build()
            .user_data(WAKE),
    )
}

pub(crate) fn push(ring: &mut IoUring, entry: squeue::Entry) -> io::Result<()> {
    unsafe { ring.submission().push(&entry) }
        .map_err(|_| io::Error::other("io_uring submission queue is full"))
}
