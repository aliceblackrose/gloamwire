/// State required to resume a Discord Gateway session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewaySession {
    session_id: String,
    resume_gateway_url: String,
    sequence: u64,
}

impl GatewaySession {
    pub(crate) fn new(
        session_id: String,
        resume_gateway_url: String,
        sequence: u64,
    ) -> Self {
        Self {
            session_id,
            resume_gateway_url,
            sequence,
        }
    }

    /// Returns the Discord Gateway session identifier.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Returns the Gateway URL Discord supplied for session resumption.
    #[must_use]
    pub fn resume_gateway_url(&self) -> &str {
        &self.resume_gateway_url
    }

    /// Returns the latest sequence number observed for this session.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub(crate) const fn update_sequence(&mut self, sequence: u64) {
        self.sequence = sequence;
    }
}
