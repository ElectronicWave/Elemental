use std::string::ToString;
use std::time::Duration;
use curl::easy::{Easy, List};

pub mod curseforge;
pub mod modrinth;

static CF_API: &str = "https://api.curseforge.com/v1/";
static MR_API: &str = "https://api.modrinth.com/";
static CF_API_SPECIAL: &str = "https://mod.mcimirror.top/curseforge/";
static MR_API_SPECIAL: &str = "https://mod.mcimirror.top/modrinth/";

#[derive(Debug)]
pub struct Discover {
    pub easy_client: Easy
}

impl Discover {
    pub fn new(url: &str) -> Self {
        let mut easy_client = Easy::new();
        easy_client.follow_location(true).unwrap();
        easy_client.timeout(Duration::from_secs(10)).unwrap();
        easy_client.url(url).unwrap();
        Discover { easy_client }
    }

    pub fn set_json_header(&mut self) {
        self.set_header("Accept", "application/json");
    }

    pub fn set_curse_key(&mut self, api_key: &str) {
        self.set_header("x-api-key", api_key);
    }

    pub fn set_rinth_key(&mut self, api_key: &str) {
        self.set_header("Authorization", api_key);
    }

    fn set_header(&mut self, header: &str, value: &str) {
        let mut headers = List::new();
        headers.append(&format!("{}: {}", header, value)).unwrap();
        self.easy_client.http_headers(headers).unwrap();
    }

    pub fn respond(&mut self) -> u32 {
        self.easy_client.response_code().unwrap()
    }

    pub fn respond_test(&mut self) -> bool {
        self.respond() == 200
    }

    pub fn get(&mut self) -> String {
        let mut data = Vec::new();
        {
            let mut transfer = self.easy_client.transfer();
            transfer.write_function(|new_data| {
                data.extend_from_slice(new_data);
                Ok(new_data.len())
            }).unwrap();
            transfer.perform().unwrap();
        }
        String::from_utf8(data).unwrap()
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