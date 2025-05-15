use std::env::var;

#[derive(Debug)]
pub struct JavaDistrubtion {
    pub path: String,
    pub info: Option<JavaDistrubtionReleaseInfo>,
}

#[derive(Debug)]
pub struct JavaDistrubtionReleaseInfo {}

impl JavaDistrubtion {
    pub fn get_all_java_distrubtion() -> Vec<Self> {
        vec![]
    }

    pub fn get_platform_java_distrubtion() -> Vec<Self> {
        todo!()
    }

    pub fn get_javahome_java_distrubtion() -> Option<Self> {
        let javahome = var("JAVA_HOME").ok();
        if let Some(path) = javahome {
            return Some(JavaDistrubtion { path, info: None });
        }

        None
    }
}

#[test]
fn javahome() {
    let p = JavaDistrubtion::get_javahome_java_distrubtion();
    println!("{:?}", p)
}
