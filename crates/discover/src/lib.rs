use std::io::{Write};
use std::string::ToString;
use curl::easy::{Easy, List};

pub mod curseforge;
pub mod modrinth;

static CF_API: &str = "https://api.curseforge.com/v1";
static MR_API: &str = "https://api.modrinth.com";
static CF_API_SPECIAL: &str = "https://mod.mcimirror.top/curseforge";
static MR_API_SPECIAL: &str = "https://mod.mcimirror.top/modrinth";

#[derive(Debug)]
pub struct Discover {
    pub easy_client: Easy
}

impl Discover {
    pub fn new(url: &str) -> Self {
        let mut easy_client = Easy::new();
        easy_client.url(url).unwrap();
        easy_client.custom_request("GET").unwrap();
        Discover { easy_client }
    }

    pub fn set_curse_key(&mut self, api_key: &str) {
        let mut headers = List::new();
        headers.append("Accept: application/json").unwrap();
        headers.append(&format!("x-api-key: {}", api_key)).unwrap();
        self.easy_client.http_headers(headers).unwrap();
    }

    pub fn set_rinth_key(&mut self, api_key: &str) {
        let mut headers = List::new();
        headers.append(&format!("{}: {}", "Authorization", api_key)).unwrap();
    }

    pub fn respond(&mut self) -> u32 {
        self.easy_client.perform().unwrap();
        self.easy_client.response_code().unwrap()
    }

    pub fn respond_test(&mut self) -> bool {
        self.respond() == 200
    }

    pub fn get(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        {
            let mut transfer = self.easy_client.transfer();
            transfer.write_function(|new_data| {
                data.extend_from_slice(new_data);
                Ok(new_data.len())
            }).unwrap();
            transfer.perform().unwrap();
        }
        data
    }
}

pub struct UrlBuilder {
    pub url: String
}

impl UrlBuilder {
    pub fn new(base_url: &str) -> Self {
        Self {
            url: base_url.to_string() + "?"
        }
    }

    pub fn add_param(mut self, key: &str, value: &str) -> Self {
        self.url.push_str(&format!("{}={}&", key, value));
        self
    }
}