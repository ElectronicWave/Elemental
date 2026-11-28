use std::fs;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::GameVersion,
    descriptors::{FORGE_DRIVER, VANILLA_DRIVER},
    driver::InstanceProbe,
};
use elemental_launcher::{
    Addon, AddonCatalogQuery, AddonDescriptor, AddonOrigin, AddonRelease, AddonSpec, DriverSpec,
    Error, InstalledAddon, LaunchOptions, PatchMode, RuntimeOptions,
};

mod common;

use common::addon::{
    build_launcher, dummy_agent_addon, sample_tweaker_addon, seed_local_vanilla_instance,
    setup_fake_java_17, test_credential,
};

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
            id: MinecraftVersionId::from("1.20.1"),
            channel: Some("release".to_owned()),
        }]))
    }

    async fn available_versions(
        &self,
        query: &AddonCatalogQuery<'_>,
    ) -> elemental_launcher::addon_kit::Result<Option<Vec<AddonRelease>>> {
        if query.driver.is_none_or(|driver| driver == VANILLA_DRIVER) {
            Ok(Some(vec![
                AddonRelease::new("1.0.0-vanilla").with_channel(Some("release".to_owned())),
            ]))
        } else {
            Ok(Some(Vec::new()))
        }
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

    let game_version = MinecraftVersionId::from("1.20.1");
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

    let game_version = MinecraftVersionId::from("1.20.1");
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
    let (agent_jar, agent_addon) =
        dummy_agent_addon(&root, "agent.jar", "acme-agent", "Acme Agent", "mode=test");
    let launcher = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap()
        .register_addon(agent_addon)
        .unwrap();

    let game_version = MinecraftVersionId::from("1.20.1");
    let prepared = launcher
        .prepare_instance("hook-inst", DriverSpec::vanilla(game_version))
        .addon(AddonSpec::new("optifine-mock"))
        .addon(AddonSpec::new("acme-agent"))
        .run()
        .await
        .unwrap();

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
    assert!(
        command
            .jvm_arguments
            .contains(&format!("-javaagent:{}=mode=test", agent_jar.display()))
    );

    let plain = build_launcher(&root, &java_home);
    let inspected = plain
        .inspect_instance("hook-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    let err = plain
        .load_instance(inspected)
        .await
        .err()
        .expect("missing runtime hook addon should fail");
    assert!(matches!(err, Error::AddonNotRegistered { id } if id == "acme-agent"));

    assert!(plain.remove_addon("hook-inst", "acme-agent").await.unwrap());
    let inspected = plain
        .inspect_instance("hook-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    let loaded = plain.load_instance(inspected).await.unwrap();
    let degraded = plain
        .build_launch_command(
            &loaded,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(degraded.main_class, "net.minecraft.launchwrapper.Launch");

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
async fn addon_registry_exposes_catalog_for_registered_addons() {
    let root = common::temp_root("elemental-addon-catalog");
    let java_home = setup_fake_java_17(&root);
    let launcher = build_launcher(&root, &java_home)
        .register_addon(CatalogStubAddon)
        .unwrap()
        .register_addon(sample_tweaker_addon())
        .unwrap();

    let mc_1_20_1 = MinecraftVersionId::from("1.20.1");

    let games = launcher
        .addons()
        .addon("catalog-stub")
        .unwrap()
        .supported_game_versions()
        .await
        .unwrap()
        .expect("catalog-stub exposes game versions");
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].id, mc_1_20_1);

    let vanilla_query = AddonCatalogQuery::new()
        .with_driver(VANILLA_DRIVER)
        .with_game_version(&mc_1_20_1);
    let vanilla_releases = launcher
        .addons()
        .addon("catalog-stub")
        .unwrap()
        .available_versions(&vanilla_query)
        .await
        .unwrap()
        .expect("catalog-stub exposes vanilla releases");
    assert_eq!(vanilla_releases.len(), 1);
    assert_eq!(vanilla_releases[0].version, "1.0.0-vanilla");

    let forge_query = AddonCatalogQuery::new()
        .with_driver(FORGE_DRIVER)
        .with_game_version(&mc_1_20_1);
    let forge_releases = launcher
        .addons()
        .addon("catalog-stub")
        .unwrap()
        .available_versions(&forge_query)
        .await
        .unwrap()
        .expect("catalog-stub filters by host driver");
    assert!(forge_releases.is_empty());

    assert!(
        launcher
            .addons()
            .addon("optifine-mock")
            .unwrap()
            .available_versions(&vanilla_query)
            .await
            .unwrap()
            .is_none()
    );

    let err = launcher
        .addons()
        .addon("unknown-addon")
        .err()
        .expect("unknown addon should error");
    assert!(matches!(err, Error::AddonNotRegistered { id } if id == "unknown-addon"));

    common::remove(&root);
}
