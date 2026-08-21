//! Per-window identity derived from a loopback peer socket.
//!
//! The Cursor `mcp-remote` bridge is stdio on the same machine, so each window
//! owns a long-lived Node process with a TCP connection into the gateway.
//! `ConnectInfo<SocketAddr>` already carries the peer port; this module maps
//! that port to the owning PID. The PID is a window key, not a folder — it
//! outlives `mcp-session-id` churn so an explicit workspace pin can stick.
//!
//! Non-loopback peers return [`None`]: a tunnelled client has no local PID to
//! resolve, and those connections keep per-session pinning.

use std::fmt;
use std::net::SocketAddr;

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use tracing::debug;

/// Owning PID of a loopback `mcp-remote` (or similar) child.
///
/// ponytail: keyed on PID alone, not PID + start time. A stale pin requires
/// the process to die *and* its PID to be reused by another `mcp-remote`
/// connected to this gateway. Upgrade path is adding start time if a misroute
/// is ever observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowKey(u32);

impl WindowKey {
    /// Wrap a raw PID. Tests construct this directly; production code goes
    /// through [`resolve_window_key`].
    pub fn from_pid(pid: u32) -> Self {
        Self(pid)
    }

    /// The OS process identifier this key represents.
    pub fn pid(self) -> u32 {
        self.0
    }

    /// Whether the owning process is still alive.
    ///
    /// A dead key must not inherit a pin — PID reuse is the documented
    /// ceiling, and a live-check is the cheap half of closing it.
    pub fn is_live(self) -> bool {
        process_is_alive(self.0)
    }
}

impl fmt::Display for WindowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pid:{}", self.0)
    }
}

/// Map a request's peer address to a window key.
///
/// Returns [`None`] when the peer is not loopback, when the OS socket table
/// has no matching TCP connection, or when the lookup itself fails. Callers
/// treat [`None`] as "no window pin for this connection."
pub fn resolve_window_key(peer: SocketAddr) -> Option<WindowKey> {
    if !peer.ip().is_loopback() {
        return None;
    }

    match pid_for_local_port(peer) {
        Some(pid) => Some(WindowKey(pid)),
        None => {
            debug!(
                peer = %peer,
                "[WindowIdentity] no owning PID for loopback peer — window pin skipped"
            );
            None
        }
    }
}

/// Look up the PID that owns a TCP socket whose *local* address matches `peer`.
///
/// From the gateway's side, `peer` is the client's source address, which is
/// the client's local bind. `mcp-remote` holds that socket.
fn pid_for_local_port(peer: SocketAddr) -> Option<u32> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let sockets = match get_sockets_info(af_flags, ProtocolFlags::TCP) {
        Ok(sockets) => sockets,
        Err(error) => {
            debug!(
                peer = %peer,
                %error,
                "[WindowIdentity] socket table read failed"
            );
            return None;
        }
    };

    for socket in sockets {
        let ProtocolSocketInfo::Tcp(tcp) = socket.protocol_socket_info else {
            continue;
        };
        if tcp.state != TcpState::Established {
            continue;
        }
        if tcp.local_port != peer.port() {
            continue;
        }
        if tcp.local_addr != peer.ip() {
            continue;
        }
        return socket.associated_pids.into_iter().next();
    }
    None
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    // SAFETY: `kill(pid, 0)` only probes existence / permission; it does not
    // deliver a signal. `EPERM` means the process exists but we cannot signal
    // it, which still counts as live for pin-eviction purposes.
    // `pid > i32::MAX` is rejected above because `u32::MAX as i32` is `-1`,
    // and `kill(-1, 0)` broadcasts to every process we can signal.
    let result = unsafe { libc::kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn CloseHandle(handle: isize) -> i32;
    }
    // SAFETY: OpenProcess / CloseHandle on a query handle; we only check
    // whether the PID exists, then close immediately.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        CloseHandle(handle);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn non_loopback_peer_has_no_window_key() {
        let peer: SocketAddr = "1.2.3.4:443".parse().unwrap();
        assert!(
            resolve_window_key(peer).is_none(),
            "tunnel / remote clients must stay on per-session pins"
        );
    }

    #[test]
    fn loopback_established_socket_resolves_to_this_process() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let server_addr = listener.local_addr().expect("listener addr");
        let _client = TcpStream::connect(server_addr).expect("connect");
        let (_accepted, peer) = listener.accept().expect("accept");

        // The OS socket table can lag the accept by a tick on a busy CI box.
        let expected = std::process::id();
        let resolved = (0..20).find_map(|_| {
            let key = resolve_window_key(peer);
            if key.is_some_and(|k| k.pid() == expected) {
                key
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
                None
            }
        });

        assert_eq!(
            resolved.map(WindowKey::pid),
            Some(expected),
            "unprivileged socket→PID lookup must work for own-user loopback"
        );
        assert!(resolved.expect("key").is_live());
    }

    #[test]
    fn dead_pid_is_not_live() {
        assert!(!WindowKey::from_pid(u32::MAX).is_live());
        assert!(!WindowKey::from_pid(0).is_live());
    }
}
