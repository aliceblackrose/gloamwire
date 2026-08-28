use std::time::Duration;

use tokio::time::{Instant, Interval, MissedTickBehavior};

use super::{OPUS_SILENCE_FRAME, VoiceError, VoiceResult};

/// Discord voice audio sample rate in hertz.
pub const DISCORD_OPUS_SAMPLE_RATE: u32 = 48_000;
/// Discord voice audio channel count.
pub const DISCORD_OPUS_CHANNELS: u8 = 2;
/// Number of silence frames Discord recommends before stopping transmission.
pub const OPUS_SILENCE_FLUSH_FRAMES: usize = 5;

/// Supported Opus frame durations at Discord's 48 kHz RTP clock rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VoiceOpusFrameDuration {
    TwoPointFiveMs,
    FiveMs,
    TenMs,
    #[default]
    TwentyMs,
    FortyMs,
    SixtyMs,
}

impl VoiceOpusFrameDuration {
    /// Returns the RTP timestamp increment for one frame at 48 kHz.
    #[must_use]
    pub const fn timestamp_step(self) -> u32 {
        match self {
            Self::TwoPointFiveMs => 120,
            Self::FiveMs => 240,
            Self::TenMs => 480,
            Self::TwentyMs => 960,
            Self::FortyMs => 1_920,
            Self::SixtyMs => 2_880,
        }
    }

    /// Returns the wall-clock duration used to pace this frame size.
    #[must_use]
    pub const fn as_duration(self) -> Duration {
        match self {
            Self::TwoPointFiveMs => Duration::from_micros(2_500),
            Self::FiveMs => Duration::from_millis(5),
            Self::TenMs => Duration::from_millis(10),
            Self::TwentyMs => Duration::from_millis(20),
            Self::FortyMs => Duration::from_millis(40),
            Self::SixtyMs => Duration::from_millis(60),
        }
    }
}

/// Borrowed encoded Opus frame ready for Discord media-layer processing.
///
/// The payload is the complete encoded Opus frame. If DAVE is enabled, DAVE
/// protection belongs after codec output and before RTP transport encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceOpusFrame<'a> {
    payload: &'a [u8],
    duration: VoiceOpusFrameDuration,
}

impl<'a> VoiceOpusFrame<'a> {
    /// Creates an encoded Opus frame with its RTP clock duration.
    pub fn new(payload: &'a [u8], duration: VoiceOpusFrameDuration) -> VoiceResult<Self> {
        if payload.is_empty() {
            return Err(VoiceError::Protocol(
                "encoded Opus frame must not be empty".to_owned(),
            ));
        }
        Ok(Self { payload, duration })
    }

    /// Creates Discord's canonical 20 ms Opus silence frame.
    #[must_use]
    pub const fn silence() -> VoiceOpusFrame<'static> {
        VoiceOpusFrame {
            payload: &OPUS_SILENCE_FRAME,
            duration: VoiceOpusFrameDuration::TwentyMs,
        }
    }

    /// Returns the encoded Opus bytes.
    #[must_use]
    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }

    /// Returns the duration represented by this encoded frame.
    #[must_use]
    pub const fn duration(self) -> VoiceOpusFrameDuration {
        self.duration
    }
}

/// Tokio-backed frame pacer for already-encoded Opus audio.
///
/// The first tick is immediately ready so callers can send the first frame
/// without an artificial startup delay. Missed ticks are skipped rather than
/// burst, preventing delayed producers from flooding several audio frames at
/// once while trying to catch up.
#[derive(Debug)]
pub struct VoiceFramePacer {
    frame_duration: VoiceOpusFrameDuration,
    interval: Interval,
}

impl VoiceFramePacer {
    #[must_use]
    pub fn new(frame_duration: VoiceOpusFrameDuration) -> Self {
        let period = frame_duration.as_duration();
        let mut interval = tokio::time::interval_at(Instant::now(), period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        Self {
            frame_duration,
            interval,
        }
    }

    /// Returns the duration this pacer schedules.
    #[must_use]
    pub const fn frame_duration(&self) -> VoiceOpusFrameDuration {
        self.frame_duration
    }

    /// Waits until the next frame may be emitted.
    pub async fn wait_for_next_frame(&mut self) {
        self.interval.tick().await;
    }
}

impl Default for VoiceFramePacer {
    fn default() -> Self {
        Self::new(VoiceOpusFrameDuration::TwentyMs)
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{Duration, timeout};

    use super::{
        DISCORD_OPUS_CHANNELS, DISCORD_OPUS_SAMPLE_RATE, OPUS_SILENCE_FLUSH_FRAMES,
        VoiceFramePacer, VoiceOpusFrame, VoiceOpusFrameDuration,
    };

    #[test]
    fn opus_frame_durations_match_48khz_rtp_clock() {
        let cases = [
            (VoiceOpusFrameDuration::TwoPointFiveMs, 120),
            (VoiceOpusFrameDuration::FiveMs, 240),
            (VoiceOpusFrameDuration::TenMs, 480),
            (VoiceOpusFrameDuration::TwentyMs, 960),
            (VoiceOpusFrameDuration::FortyMs, 1_920),
            (VoiceOpusFrameDuration::SixtyMs, 2_880),
        ];
        for (duration, expected_step) in cases {
            assert_eq!(duration.timestamp_step(), expected_step);
        }
        assert_eq!(DISCORD_OPUS_SAMPLE_RATE, 48_000);
        assert_eq!(DISCORD_OPUS_CHANNELS, 2);
        assert_eq!(OPUS_SILENCE_FLUSH_FRAMES, 5);
    }

    #[test]
    fn frame_boundary_rejects_empty_payload_and_exposes_silence() {
        assert!(VoiceOpusFrame::new(&[], VoiceOpusFrameDuration::TwentyMs).is_err());
        let silence = VoiceOpusFrame::silence();
        assert_eq!(silence.payload(), &[0xF8, 0xFF, 0xFE]);
        assert_eq!(silence.duration(), VoiceOpusFrameDuration::TwentyMs);
    }

    #[tokio::test]
    async fn pacer_allows_first_frame_immediately() {
        let mut pacer = VoiceFramePacer::new(VoiceOpusFrameDuration::SixtyMs);
        timeout(Duration::from_millis(10), pacer.wait_for_next_frame())
            .await
            .expect("first pacer tick should be immediately ready");
        assert_eq!(pacer.frame_duration(), VoiceOpusFrameDuration::SixtyMs);
    }
}
