// SPDX-License-Identifier: MIT

use super::io::Connection;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

fn pair() -> std::io::Result<(TcpStream, TcpStream)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let peer = TcpStream::connect(listener.local_addr()?)?;
    Ok((listener.accept()?.0, peer))
}

#[test]
fn idle_header_and_incomplete_body_expire() -> std::io::Result<()> {
    for prefix in [
        b"".as_slice(),
        b"POST / HTTP/1.1\r\nContent-Length: 2\r\n\r\nx",
    ] {
        let (mut server, mut peer) = pair()?;
        peer.write_all(prefix)?;
        let stop = AtomicBool::new(false);
        let start = Instant::now();
        let mut connection = Connection::new(&mut server, &stop, Duration::from_millis(60));
        assert!(super::http::read_request(&mut connection).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    Ok(())
}

#[test]
fn read_progress_does_not_extend_the_absolute_deadline() -> std::io::Result<()> {
    let (mut server, mut peer) = pair()?;
    let stop = AtomicBool::new(false);
    let mut connection = Connection::new(&mut server, &stop, Duration::from_millis(60));
    let start = Instant::now();
    loop {
        peer.write_all(b"x")?;
        let mut byte = [0];
        if let Err(error) = connection.read(&mut byte) {
            assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
            break;
        }
    }
    assert!(start.elapsed() < Duration::from_secs(2));
    Ok(())
}

#[test]
fn nonreading_peer_cannot_hold_response_writer() -> std::io::Result<()> {
    let (mut server, _peer) = pair()?;
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let mut connection = Connection::new(&mut server, &stop, Duration::from_millis(60));
    let result = connection.write_all(&vec![0; 16 * 1024 * 1024]);
    assert!(matches!(result, Err(error) if error.kind() == std::io::ErrorKind::TimedOut));
    assert!(start.elapsed() < Duration::from_secs(2));
    Ok(())
}

#[test]
fn stopping_interrupts_an_idle_connection_before_its_deadline() -> std::io::Result<()> {
    let (mut server, _peer) = pair()?;
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let (ready, started) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut connection = Connection::new(&mut server, &worker_stop, Duration::from_secs(10));
        assert!(ready.send(()).is_ok());
        connection.read(&mut [0])
    });
    assert!(started.recv_timeout(Duration::from_secs(2)).is_ok());
    let start = Instant::now();
    stop.store(true, Ordering::Release);
    assert!(
        matches!(worker.join(), Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionAborted)
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    Ok(())
}

#[test]
fn stopped_server_joins_with_an_open_unauthenticated_peer() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let mut peer = TcpStream::connect(address)?;
    peer.write_all(b"GET /health/ready HTTP/1.1\r\n")?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || {
        super::serve(
            listener,
            address.to_string(),
            b"synthetic".to_vec(),
            unused_callback,
            thread_stop,
        );
    });
    let start = Instant::now();
    stop.store(true, Ordering::Release);
    assert!(worker.join().is_ok());
    assert!(start.elapsed() < Duration::from_secs(2));
    Ok(())
}

unsafe extern "C" fn unused_callback(
    _: *const super::RuntimeRequest,
    _: *mut u8,
    _: usize,
    _: *mut usize,
) -> i32 {
    503
}
