#[derive(Debug)]
pub struct Rinth {
    api_url: String,
    api_key: String,
    special_mode: bool,
}

impl Rinth {
    pub fn new_with_default_key(special_mode: bool) -> Self {
        Self {
            api_url: "https://api.modrinth.com".to_string(),
            api_key: "CQpFgcjyqekPejv75RIe6lsKDFw5ufvA".to_string(), // Default API key
            special_mode,
        }
    }

    pub fn new(api_key: String, special_mode: bool) -> Self {
        Self {
            api_url: "https://api.modrinth.com".to_string(),
            api_key,
            special_mode,
        }
    }
}