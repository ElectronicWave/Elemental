use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use elemental_core::{
    auth::{accounts::Account, accounts::offline::OfflineAccount},
    minecraft::MinecraftVersionId,
    runtime::provider::RuntimeProvider,
};
use elemental_driver::{
    catalog::GameVersion,
    descriptors::{FORGE_DRIVER, VANILLA_DRIVER},
    driver::InstanceProbe,
};
use elemental_launcher::{
    Addon, AddonCatalogQuery, AddonDescriptor, AddonOrigin, AddonPatchMode, AddonRelease,
    AddonSpec, CustomPatchAddon, DriverSpec, Error, InstalledAddon, JvmAgentAddon, LaunchOptions,
    Launcher, MetadataPatch, PatchMode, RuntimeOptions, addon_kit::PistonMetaGenericArgument,
};

mod common;

const BASE_METADATA: &str = r#"{
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

struct FakeRuntimeProvider {
    runtime_root: PathBuf,
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

fn setup_fake_java_17(root: &Path) -> PathBuf {
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

fn seed_local_vanilla_instance(root: &Path, name: &str) -> PathBuf {
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

async fn test_credential() -> elemental_launcher::Credential {
    let account = OfflineAccount::new("Player".to_owned());
    account.credential().await.unwrap()
}

fn build_launcher(root: &Path, java_home: &Path) -> Launcher {
    Launcher::builder()
        .storage_root(root.to_path_buf())
        .runtime_providers(vec![Arc::new(FakeRuntimeProvider {
            runtime_root: java_home.to_path_buf(),
        })])
        .build()
}

fn sample_tweaker_addon() -> CustomPatchAddon {
    CustomPatchAddon::new(
        "optifine-mock",
        "OptiFine Mock",
        MetadataPatch {
            priority: 10,
            main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
            game_arguments: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain("optifine.OptiFineTweaker".to_owned()),
            ],
            ..Default::default()
        },
    )
    .with_version("HD_U_I6")
}

struct ExternalMarkerAddon;

#[async_trait]
impl Addon for ExternalMarkerAddon {
    fn descriptor(&self) -> AddonDescriptor {
        AddonDescriptor::new("external-marker", "External Marker")
    }

    async fn inspect(
        &self,
        probe: &InstanceProbe,
    ) -> elemental_launcher::addon_kit::Result<Option<InstalledAddon>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        if metadata.id == "1.20.1" {
            return Ok(Some(InstalledAddon {
                id: "external-marker".to_owned(),
                name: "External Marker".to_owned(),
                version: Some("1.0".to_owned()),
                origin: AddonOrigin::External,
                effective_patch_mode: PatchMode::Physical,
                requires_runtime_hook: false,
            }));
        }
        Ok(None)
    }
}

#[tokio::test]
async fn vpatch_applies_in_memory_without_modifying_version_json() {
    let root = common::temp_root("elemental-addon-vpatch");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "vpatch-inst");
    let original_bytes = fs::read(&json_path).unwrap();

    let launcher = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap();

    let game_version = MinecraftVersionId::try_from("1.20.1").unwrap();
    let prepared = launcher
        .prepare_instance("vpatch-inst", DriverSpec::vanilla(game_version))
        .patch_mode(PatchMode::Virtual)
        .addon(AddonSpec::new("optifine-mock"))
        .run()
        .await
        .unwrap();

    assert_eq!(fs::read(&json_path).unwrap(), original_bytes);
    assert_eq!(prepared.addons().len(), 1);
    assert_eq!(prepared.addons()[0].id, "optifine-mock");

    let inspected = launcher
        .inspect_instance("vpatch-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(inspected.addons.len(), 1);

    let loaded = launcher.load_instance(inspected).await.unwrap();
    let credential = test_credential().await;
    let command = launcher
        .build_launch_command(
            &loaded,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await
        .unwrap();

    assert_eq!(command.main_class, "net.minecraft.launchwrapper.Launch");
    assert!(
        command
            .game_arguments
            .windows(2)
            .any(|pair| pair == ["--tweakClass", "optifine.OptiFineTweaker"])
    );

    assert!(
        launcher
            .remove_addon("vpatch-inst", "optifine-mock")
            .await
            .unwrap()
    );
    let inspected_after = launcher
        .inspect_instance("vpatch-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    assert!(inspected_after.addons.is_empty());

    common::remove(&root);
}

#[tokio::test]
async fn physical_patch_and_lossless_mode_switching() {
    let root = common::temp_root("elemental-addon-ppatch");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "ppatch-inst");
    let original_bytes = fs::read(&json_path).unwrap();
    let baseline_path = root
        .join("versions")
        .join("ppatch-inst")
        .join(".elemental")
        .join("base-version.json");

    let launcher = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap();

    let game_version = MinecraftVersionId::try_from("1.20.1").unwrap();
    launcher
        .prepare_instance("ppatch-inst", DriverSpec::vanilla(game_version))
        .patch_mode(PatchMode::Virtual)
        .addon(AddonSpec::new("optifine-mock"))
        .run()
        .await
        .unwrap();
    assert!(!baseline_path.exists());

    launcher
        .set_instance_patch_mode("ppatch-inst", PatchMode::Physical)
        .await
        .unwrap();
    assert!(baseline_path.is_file());
    assert_eq!(fs::read(&baseline_path).unwrap(), original_bytes);
    let patched_disk = fs::read_to_string(&json_path).unwrap();
    assert!(patched_disk.contains("net.minecraft.launchwrapper.Launch"));
    assert!(patched_disk.contains("optifine.OptiFineTweaker"));

    launcher
        .set_instance_patch_mode("ppatch-inst", PatchMode::Virtual)
        .await
        .unwrap();
    assert!(!baseline_path.exists());
    assert_eq!(fs::read(&json_path).unwrap(), original_bytes);

    common::remove(&root);
}

#[tokio::test]
async fn third_party_runtime_hook_and_data_only_degradation() {
    let root = common::temp_root("elemental-addon-degrade");
    let java_home = setup_fake_java_17(&root);
    seed_local_vanilla_instance(&root, "hook-inst");

    let agent_jar = root.join("agent.jar");
    fs::write(&agent_jar, b"").unwrap();
    let agent_addon = JvmAgentAddon::new("acme-agent", "Acme Agent", agent_jar.clone())
        .with_version("1.0.0")
        .with_agent_option("mode=test");

    let launcher_with_addons = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap()
        .register_addon(agent_addon)
        .unwrap();

    let game_version = MinecraftVersionId::try_from("1.20.1").unwrap();
    let prepared = launcher_with_addons
        .prepare_instance("hook-inst", DriverSpec::vanilla(game_version))
        .addon(AddonSpec::new("optifine-mock"))
        .addon(AddonSpec::new("acme-agent"))
        .run()
        .await
        .unwrap();

    let credential = test_credential().await;
    let command = launcher_with_addons
        .build_launch_command(
            &prepared,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await
        .unwrap();
    let expected_agent_flag = format!("-javaagent:{}=mode=test", agent_jar.display());
    assert!(command.jvm_arguments.contains(&expected_agent_flag));

    // A launcher without the third-party addons fails explicitly for the runtime hook addon.
    let plain_launcher = build_launcher(&root, &java_home);
    let inspected = plain_launcher
        .inspect_instance("hook-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    let err = plain_launcher
        .load_instance(inspected)
        .await
        .err()
        .expect("missing runtime hook addon should fail");
    assert!(matches!(err, Error::AddonNotRegistered { id } if id == "acme-agent"));

    // Once the runtime-hook addon is removed, the data-only vpatch addon degrades gracefully.
    assert!(
        plain_launcher
            .remove_addon("hook-inst", "acme-agent")
            .await
            .unwrap()
    );
    let inspected = plain_launcher
        .inspect_instance("hook-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    let loaded = plain_launcher.load_instance(inspected).await.unwrap();
    let degraded_command = plain_launcher
        .build_launch_command(
            &loaded,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        degraded_command.main_class,
        "net.minecraft.launchwrapper.Launch"
    );

    common::remove(&root);
}

#[tokio::test]
async fn third_party_addon_inspects_external_instance() {
    let root = common::temp_root("elemental-addon-external");
    let java_home = setup_fake_java_17(&root);
    seed_local_vanilla_instance(&root, "ext-inst");

    let launcher = build_launcher(&root, &java_home)
        .register_addon(ExternalMarkerAddon)
        .unwrap();

    let report = launcher.inspect_instances().await.unwrap();
    assert_eq!(report.instances.len(), 1);
    assert_eq!(report.instances[0].addons.len(), 1);
    assert_eq!(report.instances[0].addons[0].id, "external-marker");
    assert_eq!(report.instances[0].addons[0].origin, AddonOrigin::External);

    common::remove(&root);
}

#[tokio::test]
async fn install_addons_on_existing_instance_without_driver_spec() {
    let root = common::temp_root("elemental-addon-existing");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "existing-inst");
    let original_bytes = fs::read(&json_path).unwrap();

    let launcher = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap();

    let prepared = launcher
        .install_addons("existing-inst")
        .addon(AddonSpec::new("optifine-mock"))
        .run()
        .await
        .unwrap();

    assert_eq!(fs::read(&json_path).unwrap(), original_bytes);
    assert_eq!(prepared.driver(), &VANILLA_DRIVER);
    assert_eq!(prepared.addons().len(), 1);
    assert_eq!(prepared.addons()[0].id, "optifine-mock");
    assert_eq!(
        prepared.addons()[0].effective_patch_mode,
        PatchMode::Virtual
    );

    common::remove(&root);
}

#[tokio::test]
async fn scoped_patch_modes_enforce_monotonic_rules_and_baseline_probe() {
    let root = common::temp_root("elemental-addon-scoped");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "scoped-inst");

    let agent_jar = root.join("agent.jar");
    fs::write(&agent_jar, b"").unwrap();
    let agent_addon = JvmAgentAddon::new("acme-agent", "Acme Agent", agent_jar.clone());
    let debug_addon = CustomPatchAddon::new(
        "debug-flag",
        "Debug Flag",
        MetadataPatch {
            priority: 20,
            jvm_arguments: vec![PistonMetaGenericArgument::Plain(
                "-Ddebug.only=true".to_owned(),
            )],
            ..Default::default()
        },
    );
    // Mimic a physical patch carrying com.mumfrey:liteloader to prove baseline probe avoids mis-detection.
    let tweaker_addon = CustomPatchAddon::new(
        "tweaker-pp",
        "Tweaker PP",
        MetadataPatch {
            priority: 10,
            main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
            game_arguments: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain(
                    "com.mumfrey.liteloader.launch.LiteLoaderTweaker".to_owned(),
                ),
            ],
            ..Default::default()
        },
    );

    let launcher = build_launcher(&root, &java_home)
        .register_addon(tweaker_addon)
        .unwrap()
        .register_addon(debug_addon)
        .unwrap()
        .register_addon(agent_addon)
        .unwrap();

    // Virtual instance policy must reject explicit Physical addon mode.
    let err = launcher
        .install_addons("scoped-inst")
        .patch_mode(PatchMode::Virtual)
        .addon(AddonSpec::new("tweaker-pp").with_patch_mode(AddonPatchMode::Physical))
        .run()
        .await
        .err()
        .expect("virtual instance must forbid physical addon mode");
    assert!(matches!(
        err,
        Error::Addon(elemental_launcher::addon_kit::Error::PatchModeDisallowedByInstance { .. })
    ));

    // Physical instance policy allows Physical + VirtualOnly + explicit Virtual downgrade.
    let prepared = launcher
        .install_addons("scoped-inst")
        .patch_mode(PatchMode::Physical)
        .addon(AddonSpec::new("tweaker-pp"))
        .addon(AddonSpec::new("debug-flag").with_patch_mode(AddonPatchMode::Virtual))
        .addon(AddonSpec::new("acme-agent"))
        .run()
        .await
        .unwrap();

    let addons = prepared.addons();
    assert_eq!(addons[0].effective_patch_mode, PatchMode::Physical);
    assert_eq!(addons[1].effective_patch_mode, PatchMode::Virtual);
    assert_eq!(addons[2].effective_patch_mode, PatchMode::Virtual);

    // Physical file has tweaker-pp, NOT debug-flag.
    let disk_json = fs::read_to_string(&json_path).unwrap();
    assert!(disk_json.contains("net.minecraft.launchwrapper.Launch"));
    assert!(!disk_json.contains("-Ddebug.only=true"));

    // Launch command has BOTH physical and virtual patches + agent hook.
    let credential = test_credential().await;
    let command = launcher
        .build_launch_command(
            &prepared,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(command.main_class, "net.minecraft.launchwrapper.Launch");
    assert!(
        command
            .jvm_arguments
            .contains(&"-Ddebug.only=true".to_owned())
    );
    assert!(
        command
            .jvm_arguments
            .contains(&format!("-javaagent:{}", agent_jar.display()))
    );

    // Inspect still resolves host driver as Vanilla thanks to baseline probe.
    let inspected = launcher
        .inspect_instance("scoped-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(inspected.driver.driver, VANILLA_DRIVER);

    common::remove(&root);
}

#[tokio::test]
async fn reorder_addons_and_trace_instance_patches_step_diffs() {
    let root = common::temp_root("elemental-addon-trace");
    let java_home = setup_fake_java_17(&root);
    seed_local_vanilla_instance(&root, "trace-inst");

    let addon_a = CustomPatchAddon::new(
        "addon-a",
        "Addon A",
        MetadataPatch {
            main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
            game_arguments: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain("example.TweakerA".to_owned()),
            ],
            ..Default::default()
        },
    );
    let addon_b = CustomPatchAddon::new(
        "addon-b",
        "Addon B",
        MetadataPatch {
            main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
            game_arguments: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain("example.TweakerB".to_owned()),
            ],
            java_major_version: Some(21),
            ..Default::default()
        },
    );

    let launcher = build_launcher(&root, &java_home)
        .register_addon(addon_a)
        .unwrap()
        .register_addon(addon_b)
        .unwrap();

    launcher
        .install_addons("trace-inst")
        .addon(AddonSpec::new("addon-a"))
        .addon(AddonSpec::new("addon-b"))
        .run()
        .await
        .unwrap();

    let trace = launcher.trace_instance_patches("trace-inst").await.unwrap();
    assert_eq!(trace.steps.len(), 2);
    assert_eq!(trace.steps[0].id, "addon-a");
    assert_eq!(
        trace.steps[0].main_class,
        Some((
            "net.minecraft.client.main.Main".to_owned(),
            "net.minecraft.launchwrapper.Launch".to_owned(),
        ))
    );
    assert_eq!(trace.steps[1].id, "addon-b");
    assert_eq!(trace.steps[1].main_class, None);
    assert_eq!(trace.steps[1].java_major_version, Some((17, 21)));

    let reordered = launcher
        .reorder_addons("trace-inst", &["addon-b", "addon-a"])
        .await
        .unwrap();
    assert_eq!(
        reordered
            .steps
            .iter()
            .map(|step| step.id.as_str())
            .collect::<Vec<_>>(),
        vec!["addon-b", "addon-a"]
    );
    assert_eq!(
        reordered.steps[0].main_class,
        Some((
            "net.minecraft.client.main.Main".to_owned(),
            "net.minecraft.launchwrapper.Launch".to_owned(),
        ))
    );
    assert_eq!(reordered.steps[1].main_class, None);

    common::remove(&root);
}

struct CatalogStubAddon;

#[async_trait]
impl Addon for CatalogStubAddon {
    fn descriptor(&self) -> AddonDescriptor {
        AddonDescriptor::new("catalog-stub", "Catalog Stub")
    }

    async fn supported_game_versions(
        &self,
    ) -> elemental_launcher::addon_kit::Result<Option<Vec<GameVersion>>> {
        Ok(Some(vec![GameVersion {
            id: MinecraftVersionId::try_from("1.20.1").unwrap(),
            channel: Some("release".to_owned()),
        }]))
    }

    async fn available_versions(
        &self,
        query: &AddonCatalogQuery<'_>,
    ) -> elemental_launcher::addon_kit::Result<Option<Vec<AddonRelease>>> {
        if query.driver == VANILLA_DRIVER {
            Ok(Some(vec![
                AddonRelease::new(query.game_version.clone(), "1.0.0-vanilla")
                    .with_channel(Some("release".to_owned())),
            ]))
        } else {
            Ok(Some(Vec::new()))
        }
    }
}

#[tokio::test]
async fn addon_registry_exposes_catalog_for_registered_addons() {
    let root = common::temp_root("elemental-addon-catalog");
    let java_home = setup_fake_java_17(&root);
    let launcher = build_launcher(&root, &java_home)
        .register_addon(CatalogStubAddon)
        .unwrap()
        .register_addon(sample_tweaker_addon())
        .unwrap();

    let mc_1_20_1 = MinecraftVersionId::try_from("1.20.1").unwrap();

    let games = launcher
        .addons()
        .supported_game_versions("catalog-stub")
        .await
        .unwrap()
        .expect("catalog-stub exposes game versions");
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].id, mc_1_20_1);

    let vanilla_releases = launcher
        .addons()
        .available_versions("catalog-stub", VANILLA_DRIVER, &mc_1_20_1)
        .await
        .unwrap()
        .expect("catalog-stub exposes vanilla releases");
    assert_eq!(vanilla_releases.len(), 1);
    assert_eq!(vanilla_releases[0].version, "1.0.0-vanilla");

    let forge_releases = launcher
        .addons()
        .available_versions("catalog-stub", FORGE_DRIVER, &mc_1_20_1)
        .await
        .unwrap()
        .expect("catalog-stub filters by host driver");
    assert!(forge_releases.is_empty());

    assert!(
        launcher
            .addons()
            .available_versions("optifine-mock", VANILLA_DRIVER, &mc_1_20_1)
            .await
            .unwrap()
            .is_none()
    );

    let err = launcher
        .addons()
        .available_versions("unknown-addon", VANILLA_DRIVER, &mc_1_20_1)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::AddonNotRegistered { id } if id == "unknown-addon"));

    common::remove(&root);
}
