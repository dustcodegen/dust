use std::collections::HashMap;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Receiver};

use io_uring::{IoUring, opcode, types};

use crate::http::{complete_request_len, response_head};
use crate::uring_ops::{push, submit_accept, submit_wake_read};

pub(crate) const ACCEPT: u64 = 1;
pub(crate) const WAKE: u64 = 2;
const READ: u64 = 3;
const WRITE: u64 = 4;
const OP_MASK: u64 = 7;
const READ_CHUNK: usize = 16 * 1024;
const MAX_REQUEST: usize = 1024 * 1024;

pub(crate) type RequestCallback = extern "C" fn(u64, *const u8, u64);

pub(crate) struct ResponseMessage {
    pub(crate) id: u64,
    pub(crate) status: u16,
    pub(crate) body: Box<[u8]>,
}

struct Connection {
    stream: TcpStream,
    read_chunk: Box<[u8]>,
    request: Vec<u8>,
    request_len: usize,
    pending: bool,
    write: Vec<u8>,
    written: usize,
}

impl Connection {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            read_chunk: vec![0; READ_CHUNK].into_boxed_slice(),
            request: Vec::with_capacity(READ_CHUNK),
            request_len: 0,
            pending: false,
            write: Vec::new(),
            written: 0,
        }
    }
}

pub(crate) fn run(
    mut ring: IoUring,
    listener: TcpListener,
    wake_fd: Arc<OwnedFd>,
    responses: Receiver<ResponseMessage>,
    stop: Arc<AtomicBool>,
    callback: RequestCallback,
) -> io::Result<()> {
    let mut connections = HashMap::<u64, Connection>::new();
    let mut next_id = 1u64;
    let mut wake_value = Box::new(0u64);

    submit_accept(&mut ring, listener.as_raw_fd())?;
    submit_wake_read(&mut ring, wake_fd.as_raw_fd(), &mut wake_value)?;

    loop {
        ring.submit_and_wait(1)?;
        let completions: Vec<_> = ring
            .completion()
            .map(|entry| (entry.user_data(), entry.result()))
            .collect();
        for (user_data, result) in completions {
            let operation = user_data & OP_MASK;
            let id = user_data >> 3;
            match operation {
                ACCEPT => {
                    if result >= 0 {
                        let stream = unsafe { TcpStream::from_raw_fd(result) };
                        let id = next_id;
                        next_id = next_id.wrapping_add(1).max(1);
                        connections.insert(id, Connection::new(stream));
                        submit_read(&mut ring, id, &mut connections)?;
                    } else {
                        report_completion_error("accept", 0, result);
                    }
                    if !stop.load(Ordering::Acquire) {
                        submit_accept(&mut ring, listener.as_raw_fd())?;
                    }
                }
                WAKE => {
                    while let Ok(message) = responses.try_recv() {
                        prepare_response(&mut connections, message);
                    }
                    let ready: Vec<_> = connections
                        .iter()
                        .filter_map(|(&id, connection)| {
                            (!connection.write.is_empty() && connection.written == 0).then_some(id)
                        })
                        .collect();
                    for id in ready {
                        submit_write(&mut ring, id, &mut connections)?;
                    }
                    if stop.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    submit_wake_read(&mut ring, wake_fd.as_raw_fd(), &mut wake_value)?;
                }
                READ => {
                    if result <= 0 {
                        report_completion_error("read", id, result);
                        connections.remove(&id);
                        continue;
                    }
                    let action = receive_request(&mut connections, id, result as usize);
                    match action {
                        ReadAction::More => submit_read(&mut ring, id, &mut connections)?,
                        ReadAction::Dispatch => dispatch(&mut connections, id, callback),
                        ReadAction::Reject => {
                            eprintln!("io_uring probe rejected request on connection {id}");
                            if let Some(connection) = connections.get_mut(&id) {
                                connection.pending = true;
                                connection.request_len = connection.request.len();
                            }
                            prepare_error(&mut connections, id);
                            submit_write(&mut ring, id, &mut connections)?;
                        }
                        ReadAction::Closed => {}
                    }
                }
                WRITE => {
                    if result <= 0 {
                        report_completion_error("write", id, result);
                        connections.remove(&id);
                        continue;
                    }
                    match finish_write(&mut connections, id, result as usize) {
                        WriteAction::More => submit_write(&mut ring, id, &mut connections)?,
                        WriteAction::Read => submit_read(&mut ring, id, &mut connections)?,
                        WriteAction::Dispatch => dispatch(&mut connections, id, callback),
                        WriteAction::Closed => {}
                    }
                }
                _ => {}
            }
        }
    }
}

enum ReadAction {
    More,
    Dispatch,
    Reject,
    Closed,
}

fn receive_request(
    connections: &mut HashMap<u64, Connection>,
    id: u64,
    count: usize,
) -> ReadAction {
    let Some(connection) = connections.get_mut(&id) else {
        return ReadAction::Closed;
    };
    connection
        .request
        .extend_from_slice(&connection.read_chunk[..count]);
    if connection.request.len() > MAX_REQUEST {
        return ReadAction::Reject;
    }
    match complete_request_len(&connection.request) {
        Ok(Some(length)) => {
            connection.request_len = length;
            connection.pending = true;
            ReadAction::Dispatch
        }
        Ok(None) => ReadAction::More,
        Err(()) => ReadAction::Reject,
    }
}

fn dispatch(connections: &mut HashMap<u64, Connection>, id: u64, callback: RequestCallback) {
    if let Some(connection) = connections.get(&id) {
        callback(
            id,
            connection.request.as_ptr(),
            connection.request_len as u64,
        );
    }
}

fn prepare_error(connections: &mut HashMap<u64, Connection>, id: u64) {
    let body: Box<[u8]> = b"{\"error\":\"bad request\"}".to_vec().into_boxed_slice();
    prepare_response(
        connections,
        ResponseMessage {
            id,
            status: 400,
            body,
        },
    );
}

fn prepare_response(connections: &mut HashMap<u64, Connection>, message: ResponseMessage) {
    let Some(connection) = connections.get_mut(&message.id) else {
        return;
    };
    if !connection.pending || !connection.write.is_empty() {
        return;
    }
    connection.write = response_head(message.status, message.body.len());
    connection.write.extend_from_slice(&message.body);
    connection.written = 0;
}

enum WriteAction {
    More,
    Read,
    Dispatch,
    Closed,
}

fn finish_write(connections: &mut HashMap<u64, Connection>, id: u64, count: usize) -> WriteAction {
    let Some(connection) = connections.get_mut(&id) else {
        return WriteAction::Closed;
    };
    connection.written += count;
    if connection.written < connection.write.len() {
        return WriteAction::More;
    }
    connection.write.clear();
    connection.written = 0;
    connection.request.drain(..connection.request_len);
    connection.request_len = 0;
    connection.pending = false;
    match complete_request_len(&connection.request) {
        Ok(Some(length)) => {
            connection.request_len = length;
            connection.pending = true;
            WriteAction::Dispatch
        }
        Ok(None) => WriteAction::Read,
        Err(()) => WriteAction::Closed,
    }
}

fn submit_read(
    ring: &mut IoUring,
    id: u64,
    connections: &mut HashMap<u64, Connection>,
) -> io::Result<()> {
    let connection = connections.get_mut(&id).ok_or(io::ErrorKind::NotFound)?;
    let entry = opcode::Recv::new(
        types::Fd(connection.stream.as_raw_fd()),
        connection.read_chunk.as_mut_ptr(),
        connection.read_chunk.len() as u32,
    )
    .build()
    .user_data((id << 3) | READ);
    push(ring, entry)
}

fn submit_write(
    ring: &mut IoUring,
    id: u64,
    connections: &mut HashMap<u64, Connection>,
) -> io::Result<()> {
    let connection = connections.get_mut(&id).ok_or(io::ErrorKind::NotFound)?;
    let remaining = &connection.write[connection.written..];
    let entry = opcode::Send::new(
        types::Fd(connection.stream.as_raw_fd()),
        remaining.as_ptr(),
        remaining.len() as u32,
    )
    .build()
    .user_data((id << 3) | WRITE);
    push(ring, entry)
}

fn report_completion_error(operation: &str, id: u64, result: i32) {
    if result < 0 {
        let error = io::Error::from_raw_os_error(-result);
        eprintln!("io_uring probe {operation} failed for connection {id}: {error}");
    }
}
