// SPDX-License-Identifier: MIT

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Progress never extends the socket deadline. The synchronous managed callback
/// remains responsible for its own bounded execution time.
pub(super) struct Connection<'a> {
    stream: &'a mut TcpStream,
    stop: &'a AtomicBool,
    deadline: Instant,
}

impl<'a> Connection<'a> {
    pub(super) fn new(stream: &'a mut TcpStream, stop: &'a AtomicBool, budget: Duration) -> Self {
        Self {
            stream,
            stop,
            deadline: Instant::now() + budget,
        }
    }

    fn remaining(&self) -> io::Result<Duration> {
        if self.stop.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "listener stopping",
            ));
        }
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|value| !value.is_zero())
            .map(|value| value.min(POLL_INTERVAL))
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection deadline"))
    }
}

impl Read for Connection<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        loop {
            self.stream.set_read_timeout(Some(self.remaining()?))?;
            match self.stream.read(bytes) {
                Err(error) if retryable(&error) => continue,
                result => return result,
            }
        }
    }
}

impl Write for Connection<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        loop {
            self.stream.set_write_timeout(Some(self.remaining()?))?;
            match self.stream.write(bytes) {
                Err(error) if retryable(&error) => continue,
                result => return result,
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.remaining()?;
        self.stream.flush()
    }
}

fn retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}
