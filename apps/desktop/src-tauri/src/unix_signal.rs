//! Capture who sent SIGTERM/SIGINT (`si_pid`) without fighting tokio's waiter.
//!
//! A `SA_SIGINFO` handler records the sender, then writes one byte to a socket
//! pair so the async setup task can log and exit. We do not use `tokio::signal`
//! on Unix — the last `sigaction` wins, and we need `si_pid`.

use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicI32, Ordering};

use tracing::info;

static SENDER_PID: AtomicI32 = AtomicI32::new(0);
static SIGNAL_NO: AtomicI32 = AtomicI32::new(0);
static SIGNAL_CODE: AtomicI32 = AtomicI32::new(0);
static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

/// Install the recorder and wait until SIGTERM or SIGINT arrives.
///
/// Returns `true` after the first termination signal. Returns `false` if the
/// handler could not be installed — the caller must not treat that as exit.
pub async fn wait_for_term() -> bool {
    let read_end = match install() {
        Ok(fd) => fd,
        Err(e) => {
            tracing::warn!("[Signal] Failed to install SA_SIGINFO handler: {e}");
            return false;
        }
    };

    let mut reader = match tokio::net::UnixStream::from_std(read_end) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[Signal] Failed to wrap self-pipe: {e}");
            return false;
        }
    };
    let mut buf = [0u8; 1];
    use tokio::io::AsyncReadExt;
    let _ = reader.read_exact(&mut buf).await;

    let pid = std::process::id();
    let ppid = unsafe { libc::getppid() };
    let sender = SENDER_PID.load(Ordering::SeqCst);
    let sig = SIGNAL_NO.load(Ordering::SeqCst);
    let code = SIGNAL_CODE.load(Ordering::SeqCst);
    let name = if sig == libc::SIGINT {
        "SIGINT"
    } else {
        "SIGTERM"
    };

    info!(
        pid,
        ppid,
        parent = %describe_pid(ppid),
        sender_pid = sender,
        sender = %describe_pid(sender),
        si_code = code,
        "[Signal] {name} — requesting exit"
    );
    true
}

fn install() -> std::io::Result<UnixStream> {
    let (read, write) = UnixStream::pair()?;
    read.set_nonblocking(true)?;
    write.set_nonblocking(true)?;
    PIPE_WRITE_FD.store(write.as_raw_fd(), Ordering::SeqCst);
    // Handler writes to this fd for the life of the process.
    std::mem::forget(write);

    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = record_sender as *const () as usize;
        sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(read)
}

unsafe extern "C" fn record_sender(
    sig: libc::c_int,
    info: *mut libc::siginfo_t,
    _ctx: *mut libc::c_void,
) {
    if !info.is_null() {
        // SAFETY: kernel-filled siginfo for this delivery.
        let info = unsafe { &*info };
        SENDER_PID.store(unsafe { info.si_pid() }, Ordering::SeqCst);
        SIGNAL_CODE.store(info.si_code, Ordering::SeqCst);
    }
    SIGNAL_NO.store(sig, Ordering::SeqCst);
    let fd = PIPE_WRITE_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        let byte = [1u8];
        unsafe {
            libc::write(fd, byte.as_ptr() as *const libc::c_void, 1);
        }
    }
}

/// Best-effort path/name for a live pid. `"unknown"` if pid is empty or gone.
fn describe_pid(pid: libc::pid_t) -> String {
    if pid <= 0 {
        return "unknown".to_string();
    }
    #[cfg(target_os = "macos")]
    {
        let mut buf = [0u8; 4096];
        extern "C" {
            fn proc_pidpath(pid: i32, buffer: *mut libc::c_void, buffersize: u32) -> i32;
        }
        let n = unsafe { proc_pidpath(pid, buf.as_mut_ptr() as *mut _, buf.len() as u32) };
        if n > 0 {
            return String::from_utf8_lossy(&buf[..n as usize]).into_owned();
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(path) = std::fs::read_link(format!("/proc/{pid}/exe")) {
            return path.display().to_string();
        }
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
            return comm.trim().to_string();
        }
    }
    format!("pid:{pid}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_pid_self_is_nonempty() {
        let me = std::process::id() as libc::pid_t;
        let desc = describe_pid(me);
        assert!(!desc.is_empty(), "self pid should resolve");
        assert_ne!(desc, "unknown");
    }

    #[test]
    fn describe_pid_zero_is_unknown() {
        assert_eq!(describe_pid(0), "unknown");
    }
}
