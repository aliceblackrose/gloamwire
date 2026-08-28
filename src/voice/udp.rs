use std::net::SocketAddr;

use tokio::net::{UdpSocket, lookup_host};

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
    /// Opens and connects a UDP socket to the server advertised by Voice Ready.
    pub async fn connect(ready: &VoiceReady) -> VoiceResult<Self> {
        let remote = lookup_host((ready.ip.as_str(), ready.port))
            .await?
            .next()
            .ok_or_else(|| {
                VoiceError::Protocol(format!(
                    "Voice Ready address {}:{} did not resolve",
                    ready.ip, ready.port
                ))
            })?;
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
        self.socket.send(&request).await?;

        let mut response = [0_u8; DISCOVERY_PACKET_BYTES];
        let received = self.socket.recv(&mut response).await?;
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
        if address.is_empty() {
            return Err(VoiceError::InvalidDiscoveryResponse(
                "external address was empty".to_owned(),
            ));
        }

        let port = u16::from_be_bytes([response[72], response[73]]);
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
