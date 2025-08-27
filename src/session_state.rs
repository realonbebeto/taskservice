use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::{Ready, ready};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const PROFILE_ID_KEY: &'static str = "profile_id";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_profile_id(&self, profile_id: Uuid) -> Result<(), SessionInsertError> {
        self.0.insert(Self::PROFILE_ID_KEY, profile_id)
    }

    pub fn get_profile_id(&self) -> Result<Option<Uuid>, SessionGetError> {
        self.0.get(Self::PROFILE_ID_KEY)
    }
}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
