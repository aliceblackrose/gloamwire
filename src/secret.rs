use zeroize::Zeroize;

/// Internal owned credential that redacts Debug and clears its allocation on drop.
#[derive(Clone)]
pub(crate) struct SecretString(String);

impl SecretString {
    pub(crate) const fn new(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::SecretString;

    #[test]
    fn debug_does_not_expose_credential() {
        let credential = SecretString::new("secret-sentinel".into());
        assert_eq!(format!("{credential:?}"), "[REDACTED]");
        assert_eq!(credential.expose(), "secret-sentinel");
    }
}
