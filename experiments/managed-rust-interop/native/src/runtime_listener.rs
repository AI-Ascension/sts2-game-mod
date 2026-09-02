// SPDX-License-Identifier: MIT

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, ToSocketAddrs};

pub(super) const MAX_BIND_ADDRESS_BYTES: usize = 255;

pub(super) fn valid_bind_address(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BIND_ADDRESS_BYTES
        && value
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control())
}

pub(super) fn bind(bind_address: &str, port: u16) -> std::io::Result<(TcpListener, SocketAddr)> {
    let mut addresses = (bind_address, port).to_socket_addrs()?;
    let mut last_error = None;
    for address in &mut addresses {
        match TcpListener::bind(address) {
            Ok(listener) => {
                let local_address = listener.local_addr()?;
                return Ok((listener, local_address));
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "bind address resolved to no socket addresses",
        )
    }))
}

pub(super) fn shutdown_address(address: SocketAddr) -> SocketAddr {
    match address {
        SocketAddr::V4(value) if value.ip().is_unspecified() => {
            SocketAddr::from((Ipv4Addr::LOCALHOST, value.port()))
        }
        SocketAddr::V6(value) if value.ip().is_unspecified() => {
            SocketAddr::from((Ipv6Addr::LOCALHOST, value.port()))
        }
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::{bind, shutdown_address, valid_bind_address};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn binds_a_configured_loopback_address() -> std::io::Result<()> {
        let (listener, address) = bind("127.0.0.1", 0)?;
        assert_eq!(address.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_ne!(address.port(), 0);
        drop(listener);
        Ok(())
    }

    #[test]
    fn accepts_non_empty_bind_addresses_without_whitespace() {
        assert!(valid_bind_address("127.0.0.1"));
        assert!(valid_bind_address("my-machine"));
        assert!(!valid_bind_address(""));
        assert!(!valid_bind_address("my machine"));
    }

    #[test]
    fn wakes_an_all_interfaces_listener_through_loopback() {
        let address = SocketAddr::from(([0, 0, 0, 0], 15526));
        assert_eq!(
            shutdown_address(address),
            SocketAddr::from((Ipv4Addr::LOCALHOST, 15526))
        );
    }
}
