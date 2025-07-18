use crate::{MR_API, MR_API_SPECIAL};

#[derive(Debug)]
pub struct Rinth {
    base_url: String,
    api_key: String
}

impl Rinth {
    pub fn new_with_default_key(special_mode: bool) -> Self {
        Self {
            base_url: if special_mode {
                MR_API_SPECIAL.to_string()
            } else {
                MR_API.to_string()
            },
            api_key: "CQpFgcjyqekPejv75RIe6lsKDFw5ufvA".to_string(), // Default API key
        }
    }

    pub fn new(api_key: String, special_mode: bool) -> Self {
        Self {
            base_url: if special_mode {
                MR_API_SPECIAL.to_string()
            } else {
                MR_API.to_string()
            },
            api_key
        }
    }
}