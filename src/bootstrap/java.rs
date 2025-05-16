use std::{
    collections::HashMap, env::var, fs::read_to_string, hash::RandomState, io::Result, path::Path,
};

#[derive(Debug)]
pub struct JavaDistrubtion {
    pub path: String,
    pub info: Result<JavaDistrubtionReleaseInfo>,
}

#[derive(Debug)]
pub struct JavaDistrubtionReleaseInfo {
    pub implememtor: String,
    pub implememtor_version: String,
    pub java_runtime_version: String,
}
impl JavaDistrubtionReleaseInfo {
    pub fn parse_from_string(source: String) -> Self {
        let data: HashMap<&str, &str, RandomState> =
            HashMap::from_iter(source.lines().filter_map(|e| {
                if let Some((k, v)) = e.split_once("=") {
                    return Some((k, v.trim_start_matches('"').trim_end_matches('"')));
                }
                None
            }));

        Self {
            implememtor: data
                .get("IMPLEMENTOR")
                .unwrap_or(&"IMPLEMEMTOR")
                .to_string(),
            implememtor_version: data
                .get("IMPLEMENTOR_VERSION")
                .unwrap_or(&"IMPLEMENTOR_VERSION")
                .to_string(),
            java_runtime_version: data
                .get("JAVA_RUNTIME_VERSION")
                .unwrap_or(&"JAVA_RUNTIME_VERSION")
                .to_string(),
        }
    }

    pub fn parse_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(JavaDistrubtionReleaseInfo::parse_from_string(
            read_to_string(path)?,
        ))
    }
}

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
            let releasefile = Path::new(&path.clone()).join("release");
            return Some(JavaDistrubtion {
                path,
                info: JavaDistrubtionReleaseInfo::parse_from_file(releasefile),
            });
        }

        None
    }
}

#[test]
fn javahome() {
    let p = JavaDistrubtion::get_javahome_java_distrubtion();
    println!("{:?}", p)
}
