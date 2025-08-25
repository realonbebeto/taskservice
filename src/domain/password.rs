use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct Password(String);

impl Password {
    pub fn parse(s: String) -> Result<Password, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_short = s.graphemes(true).count() < 8;

        if is_empty_or_whitespace || is_too_short {
            Err(format!("{} is not a valid profile password", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {

    use crate::domain::password::Password;
    use claims::{assert_err, assert_ok};

    #[test]
    fn valid_password() {
        let s = "1".repeat(8);
        assert_ok!(Password::parse(s));
    }

    #[test]
    fn short_password_is_rejected() {
        let s = "2".repeat(4);
        assert_err!(Password::parse(s));
    }

    #[test]
    fn empty_string_is_rejected() {
        let s = "".to_string();
        assert_err!(Password::parse(s));
    }

    #[test]
    fn whitespace_are_rejected() {
        let s = " ".to_string();
        assert_err!(Password::parse(s));
    }
}
