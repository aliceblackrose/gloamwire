/// Discord voice RTP header length in bytes.
pub const RTP_HEADER_BYTES: usize = 12;
/// Discord's Opus RTP payload type.
pub const DISCORD_OPUS_PAYLOAD_TYPE: u8 = 0x78;
/// One 20 ms Opus frame at Discord's 48 kHz sample rate.
pub const OPUS_20MS_TIMESTAMP_STEP: u32 = 960;
/// Opus silence frame Discord recommends sending five times before stopping.
pub const OPUS_SILENCE_FRAME: [u8; 3] = [0xF8, 0xFF, 0xFE];

/// Minimal RTP header used by Discord voice audio packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceRtpHeader {
    pub sequence: u16,
    pub timestamp: u32,
    pub ssrc: u32,
}

impl VoiceRtpHeader {
    /// Encodes the fixed Discord audio RTP header.
    #[must_use]
    pub fn encode(self) -> [u8; RTP_HEADER_BYTES] {
        let mut bytes = [0_u8; RTP_HEADER_BYTES];
        bytes[0] = 0x80;
        bytes[1] = DISCORD_OPUS_PAYLOAD_TYPE;
        bytes[2..4].copy_from_slice(&self.sequence.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.ssrc.to_be_bytes());
        bytes
    }

    /// Decodes a fixed Discord audio RTP header.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < RTP_HEADER_BYTES || bytes[0] != 0x80 {
            return None;
        }

        Some(Self {
            sequence: u16::from_be_bytes([bytes[2], bytes[3]]),
            timestamp: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            ssrc: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }
}

/// Wrap-aware RTP sequence/timestamp generator for Discord Opus packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceRtpSequencer {
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
    timestamp_step: u32,
}

impl VoiceRtpSequencer {
    #[must_use]
    pub const fn new(ssrc: u32) -> Self {
        Self {
            sequence: 0,
            timestamp: 0,
            ssrc,
            timestamp_step: OPUS_20MS_TIMESTAMP_STEP,
        }
    }

    /// Overrides the default RTP timestamp increment.
    #[must_use]
    pub const fn with_timestamp_step(mut self, timestamp_step: u32) -> Self {
        self.timestamp_step = timestamp_step;
        self
    }

    /// Returns the next RTP header using the configured timestamp step.
    pub fn next_header(&mut self) -> VoiceRtpHeader {
        self.next_header_with_timestamp_step(self.timestamp_step)
    }

    /// Returns the next RTP header and advances by the supplied frame step.
    ///
    /// This supports streams that intentionally mix valid Opus frame durations
    /// while keeping a single monotonically wrapping RTP sequence/timestamp state.
    pub fn next_header_with_timestamp_step(&mut self, timestamp_step: u32) -> VoiceRtpHeader {
        let header = VoiceRtpHeader {
            sequence: self.sequence,
            timestamp: self.timestamp,
            ssrc: self.ssrc,
        };
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(timestamp_step);
        header
    }
}

#[cfg(test)]
mod tests {
    use super::{OPUS_20MS_TIMESTAMP_STEP, VoiceRtpHeader, VoiceRtpSequencer};

    #[test]
    fn rtp_header_round_trips() {
        let header = VoiceRtpHeader {
            sequence: 0x1234,
            timestamp: 0x1234_5678,
            ssrc: 0xDEAD_BEEF,
        };
        assert_eq!(VoiceRtpHeader::decode(&header.encode()), Some(header));
    }

    #[test]
    fn sequencer_advances_one_twenty_ms_opus_frame() {
        let mut sequencer = VoiceRtpSequencer::new(42);
        let first = sequencer.next_header();
        let second = sequencer.next_header();
        assert_eq!(second.sequence, first.sequence + 1);
        assert_eq!(second.timestamp, first.timestamp + OPUS_20MS_TIMESTAMP_STEP);
    }

    #[test]
    fn sequencer_accepts_per_frame_timestamp_steps() {
        let mut sequencer = VoiceRtpSequencer::new(42);
        let first = sequencer.next_header_with_timestamp_step(480);
        let second = sequencer.next_header_with_timestamp_step(1_920);
        let third = sequencer.next_header_with_timestamp_step(960);
        assert_eq!(first.timestamp, 0);
        assert_eq!(second.timestamp, 480);
        assert_eq!(third.timestamp, 2_400);
    }
}
