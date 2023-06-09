//! Trivial File Transfer Protocol server implementation.
#![allow(clippy::result_unit_err)]



use crate::net::{
    self,
    time::{Duration, Instant},
    wire::{IpAddress, IpEndpoint},
};

use crate::wire::tftp::*;
use managed::ManagedSlice;

/// Maximum number of retransmissions attempted by the server before giving up.
const MAX_RETRIES: u8 = 10;

/// Interval between consecutive retries in case of no answer.
const RETRY_TIMEOUT: Duration = Duration::from_millis(200);

/// IANA port for TFTP servers.
const TFTP_PORT: u16 = 69;

/// The context over which the [`Server`] will operate.
///
/// The context allows the [`Server`] to open and close [`Handle`]s to files.
/// It does not impose any restriction on the context hierarchy: it could be a flat
/// structure or implement a directory tree. It is up to the implementors to define,
/// if required, the concepts of path separators and nesting levels.
///
/// [`Server`]: struct.Server.html
/// [`Handle`]: trait.Handle.html
pub trait Context {
    /// The `Handle` type used by this `Context`.
    type Handle: Handle;

    /// Attempts to open a file in read-only mode if `write_mode` is `false`,
    /// otherwise in read-write mode.
    ///
    /// The `filename` contained in the request packet is provided as-is: no modifications
    /// are applied besides stripping the NULL terminator.
    fn open(&mut self, filename: &str, write_mode: bool) -> Result<Self::Handle, ()>;

    /// Closes the file handle, flushing all pending changes to disk if necessary.
    fn close(&mut self, handle: Self::Handle);
}

/// An open file handle returned by a [`Context::open()`] operation.
///
/// [`Context::open()`]: trait.Context.html#tymethod.open
pub trait Handle {
    /// Pulls some bytes from this handle into the specified buffer, returning how many bytes were read.
    ///
    /// `buf` is guaranteed to be exactly 512 bytes long, the maximum packet size allowed by the protocol.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()>;

    /// Writes a buffer into this handle's buffer, returning how many bytes were written.
    ///
    /// `buf` can be anywhere from 0 to 512 bytes long.
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()>;
}

/// TFTP server.
pub struct Server {
    next_poll: Instant,
}

/// An active TFTP transfer.
pub struct Transfer<H> {
    handle: H,
    ep: IpEndpoint,

    is_write: bool,
    block_num: u16,
    // FIXME: I'd reeeally love to avoid a potential stack allocation this big :\
    last_data: Option<[u8; 512]>,
    last_len: usize,

    retries: u8,
    timeout: Instant,
}
