use const_format::formatcp;
use std::env::consts::{ARCH, OS};

pub const PLATFORM_NATIVES_DIR_NAME: &str = formatcp!("natives-{OS}-{ARCH}");

#[test]
fn test_const() {
    println!("{}", PLATFORM_NATIVES_DIR_NAME)
}
