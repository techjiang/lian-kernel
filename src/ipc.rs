//! Microkernel IPC — inter-process communication via message passing.
//!
//! In the Lian microkernel architecture, drivers and services run as separate
//! tasks and communicate through message-passing IPC rather than shared
//! memory (for isolation).

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::sync::spinlock::SpinLock;

/// Maximum message payload size.
pub const MAX_MSG_SIZE: usize = 256;

/// An IPC message.
#[derive(Clone)]
pub struct Message {
    /// Sender task ID.
    pub from: u64,
    /// Recipient endpoint ID.
    pub to: u64,
    /// Message type (user-defined).
    pub msg_type: u32,
    /// Payload.
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(from: u64, to: u64, msg_type: u32, payload: &[u8]) -> Self {
        let p = if payload.len() > MAX_MSG_SIZE {
            payload[..MAX_MSG_SIZE].to_vec()
        } else {
            payload.to_vec()
        };
        Message {
            from,
            to,
            msg_type,
            payload: p,
        }
    }
}

/// An IPC endpoint (mailbox).
pub struct Endpoint {
    pub id: u64,
    /// Pending messages queue.
    messages: Vec<Message>,
    /// Blocked task waiting on this endpoint (if any).
    blocked_task: Option<u64>,
}

impl Endpoint {
    const fn new(id: u64) -> Self {
        Endpoint {
            id,
            messages: Vec::new(),
            blocked_task: None,
        }
    }
}

/// Global IPC table.
struct IpcTable {
    endpoints: Vec<Endpoint>,
    next_id: u64,
}

impl IpcTable {
    const fn new() -> Self {
        IpcTable {
            endpoints: Vec::new(),
            next_id: 1,
        }
    }
}

static IPC: SpinLock<IpcTable> = SpinLock::new(IpcTable::new());

/// Create a new IPC endpoint and return its ID.
pub fn create_endpoint() -> u64 {
    let mut table = IPC.lock();
    let id = table.next_id;
    table.next_id += 1;
    table.endpoints.push(Endpoint::new(id));
    crate::println!("[ipc] Endpoint {} created", id);
    id
}

/// Send a message to an endpoint.
pub fn send(to_endpoint: u64, msg: Message) -> Result<(), &'static str> {
    let mut table = IPC.lock();
    for ep in table.endpoints.iter_mut() {
        if ep.id == to_endpoint {
            ep.messages.push(msg);
            return Ok(());
        }
    }
    Err("endpoint not found")
}

/// Receive a message from an endpoint (non-blocking).
pub fn recv(endpoint_id: u64) -> Option<Message> {
    let mut table = IPC.lock();
    for ep in table.endpoints.iter_mut() {
        if ep.id == endpoint_id {
            if !ep.messages.is_empty() {
                return Some(ep.messages.remove(0));
            }
        }
    }
    None
}

/// Number of active endpoints.
pub fn endpoint_count() -> usize {
    IPC.lock().endpoints.len()
}
