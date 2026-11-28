#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use elemental_core::{
    auth::{accounts::Account, accounts::offline::OfflineAccount},
    runtime::provider::RuntimeProvider,
};
use elemental_launcher::{
    Credential, CustomPatchAddon, Launcher, MetadataPatch, PatchTrace,
    addon_kit::PistonMetaGenericArgument,
};

pub const BASE_METADATA: &str = r#"{
  "arguments": {
    "game": ["--username", "${auth_player_name}"],
    "jvm": ["-Xmx1G"]
  },
  "assetIndex": {
    "id": "1.20",
    "sha1": "",
    "size": 0,
    "totalSize": 0,
    "url": "https://example.invalid/assets.json"
  },
  "assets": "1.20",
  "complianceLevel": 0,
  "downloads": {
    "client": {
      "sha1": "",
      "size": 0,
      "url": "https://example.invalid/client.jar"
    }
  },
  "id": "1.20.1",
  "javaVersion": {
    "component": "java-runtime-gamma",
    "majorVersion": 17
  },
  "libraries": [],
  "mainClass": "net.minecraft.client.main.Main",
  "minimumLauncherVersion": 21,
  "releaseTime": "2024-01-01T00:00:00Z",
  "time": "2024-01-01T00:00:00Z",
  "type": "release"
}"#;

pub struct FakeRuntimeProvider {
    pub runtime_root: PathBuf,
}

#[async_trait]
impl RuntimeProvider for FakeRuntimeProvider {
    async fn list(&self) -> Vec<PathBuf> {
        vec![self.runtime_root.clone()]
    }

    fn name(&self) -> &'static str {
        "FakeTestRuntime"
    }
}

pub fn setup_fake_java_17(root: &Path) -> PathBuf {
    let java_home = root.join("fake-jdk-17");
    let bin = java_home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    #[cfg(windows)]
    let exe_name = "java.exe";
    #[cfg(not(windows))]
    let exe_name = "java";
    fs::write(bin.join(exe_name), b"").unwrap();
    fs::write(java_home.join("release"), "JAVA_VERSION=\"17.0.1\"\n").unwrap();
    java_home
}

pub fn seed_local_vanilla_instance(root: &Path, name: &str) -> PathBuf {
    let version_dir = root.join("versions").join(name);
    let assets_indexes = root.join("assets").join("indexes");
    fs::create_dir_all(&version_dir).unwrap();
    fs::create_dir_all(&assets_indexes).unwrap();
    let json_path = version_dir.join(format!("{name}.json"));
    fs::write(&json_path, BASE_METADATA).unwrap();
    fs::write(version_dir.join(format!("{name}.jar")), b"").unwrap();
    fs::write(assets_indexes.join("1.20.json"), r#"{"objects":{}}"#).unwrap();
    json_path
}

pub async fn test_credential() -> Credential {
    let account = OfflineAccount::new("Player".to_owned());
    account.credential().await.unwrap()
}

pub fn build_launcher(root: &Path, java_home: &Path) -> Launcher {
    Launcher::builder()
        .storage_root(root.to_path_buf())
        .runtime_providers(vec![Arc::new(FakeRuntimeProvider {
            runtime_root: java_home.to_path_buf(),
        })])
        .build()
}

pub fn sample_tweaker_addon() -> CustomPatchAddon {
    tweaker_addon(
        "optifine-mock",
        "OptiFine Mock",
        10,
        "optifine.OptiFineTweaker",
    )
    .with_version("HD_U_I6")
}

pub fn tweaker_addon(
    id: &'static str,
    name: &'static str,
    priority: i32,
    tweaker: &str,
) -> CustomPatchAddon {
    CustomPatchAddon::new(
        id,
        name,
        MetadataPatch {
            priority,
            main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
            game_arguments: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain(tweaker.to_owned()),
            ],
            ..Default::default()
        },
    )
}

pub fn dummy_agent_addon(
    root: &Path,
    jar_name: &str,
    id: &'static str,
    name: &'static str,
    option: &str,
) -> (PathBuf, elemental_launcher::JvmAgentAddon) {
    let agent_jar = root.join(jar_name);
    fs::write(&agent_jar, b"").unwrap();
    let addon = elemental_launcher::JvmAgentAddon::new(id, name, agent_jar.clone())
        .with_version("1.0.0")
        .with_agent_option(option);
    (agent_jar, addon)
}

pub fn jvm_flag_addon(
    id: &'static str,
    name: &'static str,
    priority: i32,
    flag: &str,
) -> CustomPatchAddon {
    CustomPatchAddon::new(
        id,
        name,
        MetadataPatch {
            priority,
            jvm_arguments: vec![PistonMetaGenericArgument::Plain(flag.to_owned())],
            ..Default::default()
        },
    )
}

pub fn step_ids(trace: &PatchTrace) -> Vec<&str> {
    trace.steps.iter().map(|step| step.id.as_str()).collect()
}
