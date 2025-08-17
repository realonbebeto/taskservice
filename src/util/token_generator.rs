use rand::{Rng, distr::Alphanumeric};

pub fn generate_profile_token() -> String {
    let mut rng = rand::rng();

    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
