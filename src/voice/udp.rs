use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use tokio::net::UdpSocket;

use super::{VoiceError, VoiceReady, VoiceResult};

const DISCOVERY_PACKET_BYTES: usize = 74;
const DISCOVERY_PAYLOAD_BYTES: u16 = 70;
const DISCOVERY_REQUEST_TYPE: u16 = 1;
const DISCOVERY_RESPONSE_TYPE: u16 = 2;

/// External UDP address discovered through Discord's voice server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceUdpDiscovery {
    pub address: String,
    pub port: u16,
}

/// Policy for the numeric UDP destination supplied by Voice Ready.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VoiceUdpDestinationPolicy {
    /// Require a public unicast address. Reject special-use ranges.
    #[default]
    PublicOnly,
    /// Also permit private, unique-local and loopback addresses for trusted
    /// infrastructure or fixtures. Never use this for untrusted voice servers.
    AllowPrivate,
}

impl VoiceUdpDestinationPolicy {
    fn permits(self, address: IpAddr) -> bool {
        match address {
            IpAddr::V4(ip) => self.permits_v4(ip),
            IpAddr::V6(ip) => self.permits_v6(ip),
        }
    }

    fn permits_v4(self, ip: Ipv4Addr) -> bool {
        if ip.is_private() || ip.is_loopback() {
            return self == Self::AllowPrivate;
        }
        let [a, b, c, _] = ip.octets();
        !(a == 0
            || a >= 224
            || (a == 100 && (64..=127).contains(&b))
            || (a == 169 && b == 254)
            || (a == 192 && b == 0 && (c == 0 || c == 2))
            || (a == 192 && b == 88 && c == 99)
            || (a == 198 && (b == 18 || b == 19))
            || (a == 198 && b == 51 && c == 100)
            || (a == 203 && b == 0 && c == 113))
    }

    fn permits_v6(self, ip: Ipv6Addr) -> bool {
        if let Some(mapped) = ip.to_ipv4_mapped() {
            return self.permits_v4(mapped);
        }
        if ip.is_loopback() || ip.is_unique_local() {
            return self == Self::AllowPrivate;
        }
        let [a, b, ..] = ip.segments();
        // Allow global unicast, excluding IETF special-purpose, documentation
        // and transition ranges that can encode unexpected IPv4 destinations.
        (0x2000..=0x3fff).contains(&a)
            && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8))
            && a != 0x2002
            && a != 0x3fff
    }
}

/// Connected UDP socket for Discord voice transport.
pub struct VoiceUdpSocket {
    socket: UdpSocket,
    remote: SocketAddr,
    ssrc: u32,
}

impl std::fmt::Debug for VoiceUdpSocket {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceUdpSocket")
            .field("remote", &self.remote)
            .field("ssrc", &self.ssrc)
            .finish_non_exhaustive()
    }
}

impl VoiceUdpSocket {
    /// Connects only to a public numeric unicast address advertised by Voice Ready.
    pub async fn connect(ready: &VoiceReady) -> VoiceResult<Self> {
        Self::connect_with_policy(ready, VoiceUdpDestinationPolicy::PublicOnly).await
    }

    /// Connects using an explicitly selected destination policy. Hostnames are
    /// rejected, avoiding DNS rebinding and unexpected resolver activity.
    pub async fn connect_with_policy(
        ready: &VoiceReady,
        policy: VoiceUdpDestinationPolicy,
    ) -> VoiceResult<Self> {
        let ip = ready.ip.parse::<IpAddr>().map_err(|_| {
            VoiceError::Protocol("Voice Ready requires a numeric UDP address".into())
        })?;
        if ready.port == 0 || !policy.permits(ip) {
            return Err(VoiceError::Protocol(
                "Voice Ready UDP destination rejected by policy".into(),
            ));
        }
        let remote = SocketAddr::new(ip, ready.port);
        let bind = if remote.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = UdpSocket::bind(bind).await?;
        socket.connect(remote).await?;

        Ok(Self {
            socket,
            remote,
            ssrc: ready.ssrc,
        })
    }

    /// Performs Discord's UDP IP discovery handshake.
    pub async fn discover(&self) -> VoiceResult<VoiceUdpDiscovery> {
        let mut request = [0_u8; DISCOVERY_PACKET_BYTES];
        request[..2].copy_from_slice(&DISCOVERY_REQUEST_TYPE.to_be_bytes());
        request[2..4].copy_from_slice(&DISCOVERY_PAYLOAD_BYTES.to_be_bytes());
        request[4..8].copy_from_slice(&self.ssrc.to_be_bytes());
        // One extra byte makes an oversized datagram distinguishable from a valid reply.
        let mut response = [0_u8; DISCOVERY_PACKET_BYTES + 1];
        let timeout = std::time::Duration::from_secs(3);
        let mut received = None;
        for _ in 0..3 {
            tokio::time::timeout(timeout, self.socket.send(&request))
                .await
                .map_err(|_| VoiceError::Timeout(timeout))??;
            if let Ok(result) = tokio::time::timeout(timeout, self.socket.recv(&mut response)).await
            {
                received = Some(result?);
                break;
            }
        }
        let received = received.ok_or(VoiceError::Timeout(timeout * 3))?;
        if received != DISCOVERY_PACKET_BYTES {
            return Err(VoiceError::InvalidDiscoveryResponse(format!(
                "expected {DISCOVERY_PACKET_BYTES} bytes, received {received}"
            )));
        }

        let packet_type = u16::from_be_bytes([response[0], response[1]]);
        let length = u16::from_be_bytes([response[2], response[3]]);
        let ssrc = u32::from_be_bytes([response[4], response[5], response[6], response[7]]);
        if packet_type != DISCOVERY_RESPONSE_TYPE {
            return Err(VoiceError::InvalidDiscoveryResponse(format!(
                "expected response type {DISCOVERY_RESPONSE_TYPE}, received {packet_type}"
            )));
        }
        if length != DISCOVERY_PAYLOAD_BYTES {
            return Err(VoiceError::InvalidDiscoveryResponse(format!(
                "expected payload length {DISCOVERY_PAYLOAD_BYTES}, received {length}"
            )));
        }
        if ssrc != self.ssrc {
            return Err(VoiceError::InvalidDiscoveryResponse(format!(
                "expected SSRC {}, received {ssrc}",
                self.ssrc
            )));
        }

        let address_bytes = &response[8..72];
        let address_end = address_bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(address_bytes.len());
        let address = std::str::from_utf8(&address_bytes[..address_end])
            .map_err(|_| {
                VoiceError::InvalidDiscoveryResponse(
                    "external address was not valid UTF-8".to_owned(),
                )
            })?
            .to_owned();
        if address.parse::<std::net::IpAddr>().is_err() {
            return Err(VoiceError::InvalidDiscoveryResponse(
                "external address was not an IP address".to_owned(),
            ));
        }

        let port = u16::from_be_bytes([response[72], response[73]]);
        if port == 0 {
            return Err(VoiceError::InvalidDiscoveryResponse(
                "external port was zero".into(),
            ));
        }
        Ok(VoiceUdpDiscovery { address, port })
    }

    /// Returns the Discord voice server this UDP socket is connected to.
    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.remote
    }

    /// Returns the local UDP socket address selected by the operating system.
    pub fn local_addr(&self) -> VoiceResult<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    /// Sends one already-encrypted RTP packet to Discord.
    pub async fn send(&self, packet: &[u8]) -> VoiceResult<usize> {
        Ok(self.socket.send(packet).await?)
    }

    /// Receives one UDP packet from Discord.
    pub async fn recv(&self, buffer: &mut [u8]) -> VoiceResult<usize> {
        Ok(self.socket.recv(buffer).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::{DISCOVERY_PACKET_BYTES, DISCOVERY_PAYLOAD_BYTES, DISCOVERY_RESPONSE_TYPE};

    #[test]
    fn discovery_wire_constants_match_discord() {
        assert_eq!(DISCOVERY_PACKET_BYTES, 74);
        assert_eq!(DISCOVERY_PAYLOAD_BYTES, 70);
        assert_eq!(DISCOVERY_RESPONSE_TYPE, 2);
    }
}

#[cfg(test)]
mod destination_tests {
    use super::*;

    #[test]
    fn public_policy_rejects_local_and_special_use_addresses() {
        let policy = VoiceUdpDestinationPolicy::PublicOnly;
        for ip in [
            "0.0.0.0",
            "127.0.0.1",
            "10.0.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "192.0.2.1",
            "224.0.0.1",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "2002:7f00:1::",
            "ff02::1",
        ] {
            assert!(!policy.permits(ip.parse().unwrap()), "{ip}");
        }
        assert!(policy.permits("8.8.8.8".parse().unwrap()));
        assert!(policy.permits("2606:4700::1111".parse().unwrap()));
    }

    #[test]
    fn private_policy_is_explicit_and_still_rejects_link_local() {
        let policy = VoiceUdpDestinationPolicy::AllowPrivate;
        assert!(policy.permits("127.0.0.1".parse().unwrap()));
        assert!(policy.permits("10.0.0.1".parse().unwrap()));
        assert!(policy.permits("::1".parse().unwrap()));
        assert!(!policy.permits("169.254.169.254".parse().unwrap()));
        assert!(!policy.permits("ff02::1".parse().unwrap()));
    }
}
