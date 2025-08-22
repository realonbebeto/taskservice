use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::ValidateEmail;

#[derive(Deserialize, Serialize, Debug, ToSchema, Clone)]

pub struct ProfileEmail(String);

impl ProfileEmail {
    pub fn parse(s: String) -> Result<ProfileEmail, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid email.", s))
        }
        // TODO: add validation
    }
}

impl AsRef<str> for ProfileEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProfileEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::ProfileEmail;
    use claims::assert_err;
    use fake::Fake;
    use fake::faker::internet::en::SafeEmail;
    use rand::SeedableRng;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(ProfileEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(ProfileEmail::parse(email));
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@adomain.com".to_string();
        assert_err!(ProfileEmail::parse(email));
    }
    #[derive(Debug, Clone)]

    struct ValidateEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidateEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let seed: u64 = quickcheck::Arbitrary::arbitrary(g);
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

            let email: String = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidateEmailFixture) -> bool {
        ProfileEmail::parse(valid_email.0).is_ok()
    }
}
