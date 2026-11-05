use crate::auth::credential::UserCredential;
use crate::auth::error::Result;

pub trait Authorizer {
    fn authorize(&self) -> impl Future<Output = Result<UserCredential>>;
    fn name() -> &'static str {
        "Authorizer"
    }
}
