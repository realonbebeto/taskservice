use std::ops::Deref;

use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct ProfileId(pub Uuid);

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for ProfileId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
