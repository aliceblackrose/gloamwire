/// Reusable before/after cursor pagination for Discord collection endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination<I> {
    pub before: Option<I>,
    pub after: Option<I>,
    pub limit: Option<u16>,
}

impl<I> Default for Pagination<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I> Pagination<I> {
    /// Creates an empty pagination request using Discord endpoint defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            before: None,
            after: None,
            limit: None,
        }
    }

    /// Sets the exclusive cursor before which results are returned.
    #[must_use]
    pub fn before(mut self, cursor: I) -> Self {
        self.before = Some(cursor);
        self.after = None;
        self
    }

    /// Sets the exclusive cursor after which results are returned.
    #[must_use]
    pub fn after(mut self, cursor: I) -> Self {
        self.after = Some(cursor);
        self.before = None;
        self
    }

    /// Sets the endpoint-specific maximum number of results to request.
    #[must_use]
    pub const fn limit(mut self, limit: u16) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::model::MessageId;

    use super::Pagination;

    #[test]
    fn before_and_after_are_mutually_exclusive() {
        let pagination = Pagination::new()
            .before(MessageId::new(1))
            .after(MessageId::new(2));

        assert!(pagination.before.is_none());
        assert_eq!(pagination.after.expect("after").get(), 2);
    }

    #[test]
    fn default_does_not_require_a_default_cursor() {
        let pagination = Pagination::<MessageId>::default();

        assert_eq!(pagination, Pagination::new());
    }
}
