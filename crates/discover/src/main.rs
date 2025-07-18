use curl::easy::{Easy};
use elemental_discover::curseforge::Curse;

fn main() {
    let curse = Curse::new_with_default_key(false);
    //curse.get_mod(238222);
    println!("{:?}", curse.get_mod(238222))
}