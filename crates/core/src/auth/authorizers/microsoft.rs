use crate::auth::authorizer::Authorizer;
use crate::auth::credential::UserCredential;
use anyhow::Result;

pub struct MicrosoftAuthorizer {
    pub client_id: String,
}

impl Authorizer for MicrosoftAuthorizer {
    async fn authorize(&self) -> Result<UserCredential> {
        // Implementation for Microsoft authorization
        todo!()
    }

    fn name() -> &'static str {
        "Microsoft"
    }
}
