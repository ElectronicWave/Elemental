use std::{fs, path::Path};

use elemental_launcher::{
    AddonPatchMode, AddonSpec, CustomPatchAddon, Error, Launcher, MetadataPatch, PatchMode,
    addon_kit::{NamedMetadataPatch, PistonMetaGenericArgument},
};

mod common;

use common::addon::{
    build_launcher, dummy_agent_addon, jvm_flag_addon, sample_tweaker_addon,
    seed_local_vanilla_instance, setup_fake_java_17, step_ids, test_credential, tweaker_addon,
};

fn build_modify_launcher(root: &Path, java_home: &Path) -> Launcher {
    let addon_a = tweaker_addon("addon-a", "Addon A", 0, "example.TweakerA");
    let addon_b = CustomPatchAddon::new(
        "addon-b",
        "Addon B",
        MetadataPatch {
            main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
            java_major_version: Some(21),
            ..Default::default()
        },
    );

    build_launcher(root, java_home)
        .register_addon(addon_a)
        .unwrap()
        .register_addon(addon_b)
        .unwrap()
}

async fn setup_modify_instance(launcher: &Launcher, name: &str) {
    launcher
        .install_addons(name)
        .addon(AddonSpec::new("addon-a"))
        .addon(AddonSpec::new("addon-b"))
        .run()
        .await
        .unwrap();
}

#[tokio::test]
async fn launch_scoped_addons_apply_metadata_patch_and_runtime_hook_ephemerally() {
    let root = common::temp_root("elemental-addon-launch-scoped");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "launch-scoped-inst");
    let original_bytes = fs::read(&json_path).unwrap();

    let (agent_jar, agent_addon) = dummy_agent_addon(
        &root,
        "launch-agent.jar",
        "ephemeral-agent",
        "Ephemeral Agent",
        "scope=launch",
    );
    let launcher = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap()
        .register_addon(agent_addon)
        .unwrap();

    let inspected = launcher
        .inspect_instance("launch-scoped-inst".to_owned())
        .await
        .unwrap()
        .unwrap();
    let loaded = launcher.load_instance(inspected).await.unwrap();
    let credential = test_credential().await;
    let plain = launcher
        .launch_prepared(&loaded, &credential)
        .build_command()
        .await
        .unwrap();
    assert_eq!(plain.main_class, "net.minecraft.client.main.Main");

    let extra = jvm_flag_addon("extra-jvm", "Extra JVM", 0, "-Dephemeral.flag=true");
    let scoped = launcher
        .launch_prepared(&loaded, &credential)
        .addon(AddonSpec::new("optifine-mock"))
        .addon(AddonSpec::new("ephemeral-agent"))
        .with_addon(extra, None)
        .build_command()
        .await
        .unwrap();
    assert_eq!(scoped.main_class, "net.minecraft.launchwrapper.Launch");
    let has_tweak = scoped
        .game_arguments
        .windows(2)
        .any(|p| p == ["--tweakClass", "optifine.OptiFineTweaker"]);
    assert!(has_tweak);
    assert!(
        scoped
            .jvm_arguments
            .iter()
            .any(|a| a == "-Dephemeral.flag=true")
    );
    let agent_flag = format!("-javaagent:{}=scope=launch", agent_jar.display());
    assert!(scoped.jvm_arguments.contains(&agent_flag));

    assert_eq!(fs::read(&json_path).unwrap(), original_bytes);
    let persisted = launcher
        .instance_addons("launch-scoped-inst")
        .await
        .unwrap();
    assert!(persisted.is_empty());

    common::remove(&root);
}

#[tokio::test]
async fn launch_scoped_addons_sort_across_scopes_and_reject_physical_mode() {
    let root = common::temp_root("elemental-addon-launch-order");
    let java_home = setup_fake_java_17(&root);
    seed_local_vanilla_instance(&root, "launch-order-inst");

    let persistent_a = CustomPatchAddon::new(
        "persistent-a",
        "Persistent A",
        MetadataPatch {
            priority: 20,
            main_class: Some("example.PersistentMain".to_owned()),
            ..Default::default()
        },
    );
    let launcher = build_launcher(&root, &java_home)
        .register_addon(sample_tweaker_addon())
        .unwrap()
        .register_addon(persistent_a)
        .unwrap();

    let prepared_persistent = launcher
        .install_addons("launch-order-inst")
        .addon(AddonSpec::new("persistent-a"))
        .run()
        .await
        .unwrap();

    let launch_before = CustomPatchAddon::new(
        "launch-before",
        "Launch Before",
        MetadataPatch {
            priority: 100,
            before: vec!["persistent-a".to_owned()],
            main_class: Some("example.PersistentMain".to_owned()),
            ..Default::default()
        },
    );
    let credential = test_credential().await;
    let trace = launcher
        .launch_prepared(&prepared_persistent, &credential)
        .with_addon(launch_before, None)
        .trace()
        .await
        .unwrap();
    assert_eq!(step_ids(&trace), vec!["launch-before", "persistent-a"]);

    let err = launcher
        .launch_prepared(&prepared_persistent, &credential)
        .addon(AddonSpec::new("optifine-mock").with_patch_mode(AddonPatchMode::Physical))
        .build_command()
        .await
        .expect_err("physical mode must be rejected for launch-scoped addons");
    assert!(matches!(
        err,
        Error::Addon(elemental_launcher::addon_kit::Error::PatchModeDisallowedByInstance { .. })
    ));

    common::remove(&root);
}

#[tokio::test]
async fn addon_modify_previews_reorders_and_modes_in_memory() {
    let root = common::temp_root("elemental-addon-modify-preview");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "modify-inst");
    let launcher = build_modify_launcher(&root, &java_home);
    setup_modify_instance(&launcher, "modify-inst").await;

    let state_path = root
        .join("versions")
        .join("modify-inst")
        .join(".elemental")
        .join("addons.json");
    let original_json = fs::read(&json_path).unwrap();
    let original_state = fs::read(&state_path).unwrap();

    let mut modify = launcher
        .modify_instance_addons("modify-inst")
        .await
        .unwrap();
    assert!(!modify.is_dirty());
    assert_eq!(modify.patch_mode(), PatchMode::Virtual);
    assert_eq!(
        step_ids(&modify.trace().unwrap()),
        vec!["addon-a", "addon-b"]
    );

    let reordered_trace = modify.reorder(&["addon-b", "addon-a"]).unwrap();
    assert!(modify.is_dirty());
    assert_eq!(step_ids(&reordered_trace), vec!["addon-b", "addon-a"]);
    let draft_ids: Vec<String> = modify.addons().into_iter().map(|a| a.id).collect();
    assert_eq!(draft_ids, vec!["addon-b", "addon-a"]);

    let err = modify
        .set_addon_patch_mode("addon-a", AddonPatchMode::Physical)
        .expect_err("physical addon mode under virtual instance should fail");
    assert!(matches!(
        err,
        elemental_launcher::addon_kit::Error::PatchModeDisallowedByInstance { .. }
    ));

    modify.set_instance_patch_mode(PatchMode::Physical).unwrap();
    modify
        .set_addon_patch_mode("addon-a", AddonPatchMode::Physical)
        .unwrap();
    assert_eq!(modify.patch_mode(), PatchMode::Physical);
    let addon_a = modify
        .addons()
        .into_iter()
        .find(|a| a.id == "addon-a")
        .unwrap();
    assert_eq!(addon_a.effective_patch_mode, PatchMode::Physical);

    let extra_trace = modify
        .trace_with_extra(&[NamedMetadataPatch {
            id: "ephemeral-extra".to_owned(),
            effective_mode: PatchMode::Virtual,
            patch: MetadataPatch {
                before: vec!["addon-a".to_owned()],
                jvm_arguments: vec![PistonMetaGenericArgument::Plain(
                    "-Dpreview.extra=true".into(),
                )],
                ..Default::default()
            },
        }])
        .unwrap();
    assert_eq!(
        step_ids(&extra_trace),
        vec!["addon-b", "ephemeral-extra", "addon-a"]
    );
    assert_eq!(fs::read(&json_path).unwrap(), original_json);
    assert_eq!(fs::read(&state_path).unwrap(), original_state);

    common::remove(&root);
}

#[tokio::test]
async fn addon_modify_discard_restores_draft_and_commit_persists() {
    let root = common::temp_root("elemental-addon-modify-commit");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "modify-commit-inst");
    let launcher = build_modify_launcher(&root, &java_home);
    setup_modify_instance(&launcher, "modify-commit-inst").await;

    let state_path = root
        .join("versions")
        .join("modify-commit-inst")
        .join(".elemental")
        .join("addons.json");
    let original_json = fs::read(&json_path).unwrap();
    let original_state = fs::read(&state_path).unwrap();

    let mut modify = launcher
        .modify_instance_addons("modify-commit-inst")
        .await
        .unwrap();
    modify.reorder(&["addon-b", "addon-a"]).unwrap();
    modify.set_instance_patch_mode(PatchMode::Physical).unwrap();

    let discarded = modify.discard().unwrap();
    assert!(!modify.is_dirty());
    assert_eq!(modify.patch_mode(), PatchMode::Virtual);
    assert_eq!(step_ids(&discarded), vec!["addon-a", "addon-b"]);
    assert_eq!(fs::read(&json_path).unwrap(), original_json);
    assert_eq!(fs::read(&state_path).unwrap(), original_state);

    modify.reorder(&["addon-b", "addon-a"]).unwrap();
    let committed = modify.commit().await.unwrap();
    assert!(!modify.is_dirty());
    assert_eq!(step_ids(&committed), vec!["addon-b", "addon-a"]);

    let disk_trace = launcher
        .trace_instance_patches("modify-commit-inst")
        .await
        .unwrap();
    assert_eq!(step_ids(&disk_trace), vec!["addon-b", "addon-a"]);

    common::remove(&root);
}
