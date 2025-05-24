use md5::{Digest, Md5};
use uuid::Uuid;

pub fn player_uuid(name: impl Into<String>) -> String {
    let offline_name = format!("OfflinePlayer:{}", name.into());
    let mut hasher = Md5::new();
    hasher.update(offline_name);
    let mut buffer = hasher.finalize();
    buffer[6] = (buffer[6] & 0x0f) | 0x30;
    buffer[8] = (buffer[8] & 0x3f) | 0x80;
    Uuid::from_bytes(<[u8; 16]>::try_from(buffer.to_vec()).unwrap()).to_string()
}

#[test]
fn test_uuid() {
    println!("{}", player_uuid("MinecraftWiki"));
}
