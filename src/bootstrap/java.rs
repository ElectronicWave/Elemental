use std::{
    collections::HashMap, env::var, fs::read_to_string, hash::RandomState, io::Result, path::Path,
    process::Command,
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

    pub fn parse_from_release_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::parse_from_string(read_to_string(path)?))
    }

    pub fn parse_from_executable_cmdl(executable: String) -> Result<Self> {
        let cmdl = Command::new(executable)
            .arg("-XshowSettings:properties")
            .arg("-version")
            .output()?;
        let output = String::from_utf8(cmdl.stderr)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        //TODO Adapt More Java Ver Here

        // java.vm.version
        // java.vm.vendor
        // java.vendor.version
        let mut implememtor = String::new();
        let mut implememtor_version = String::new();
        let mut java_runtime_version = String::new();

        for line in output.lines() {
            let trimed = line.trim();
            if trimed.starts_with("java.vm.vendor = ") {
                implememtor = trimed.trim_start_matches("java.vm.vendor = ").to_string();
            } else if trimed.starts_with("java.vm.version = ") {
                java_runtime_version = trimed.trim_start_matches("java.vm.version = ").to_string();
            } else if trimed.starts_with("java.vendor.version") {
                implememtor_version = trimed
                    .trim_start_matches("java.vendor.version = ")
                    .to_string();
            }
        }
        Ok(Self {
            implememtor,
            implememtor_version,
            java_runtime_version,
        })
    }

    pub fn try_parse<P: AsRef<Path>>(release: P, executable: String) -> Result<Self> {
        let result = Self::parse_from_release_file(release);
        if result.is_ok() {
            return result;
        }

        Self::parse_from_executable_cmdl(executable)
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
            let info = JavaDistrubtionReleaseInfo::try_parse(
                releasefile,
                Self::get_executable_file_path_from_path(&path)?,
            );
            return Some(Self { path, info });
        }

        None
    }

    pub fn get_executable_file_path(&self) -> Option<String> {
        Self::get_executable_file_path_from_path(&self.path)
    }

    pub(crate) fn get_executable_file_path_from_path(path: &str) -> Option<String> {
        let mut filename = "java".to_owned();

        #[cfg(windows)]
        {
            filename = format!("{}.exe", filename);
        };

        let executable = Path::new(path).join("bin").join(filename);

        if executable.exists() {
            Some(executable.to_string_lossy().to_string())
        } else {
            None
        }
    }
}

#[test]
fn javahome() {
    let p = JavaDistrubtion::get_javahome_java_distrubtion();

    println!(
        "{:?}",
        JavaDistrubtionReleaseInfo::parse_from_executable_cmdl(
            p.unwrap().get_executable_file_path().unwrap(),
        )
        .unwrap()
    );
}
