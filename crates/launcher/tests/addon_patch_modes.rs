use std::fs;

use elemental_driver::descriptors::VANILLA_DRIVER;
use elemental_launcher::{
    AddonPatchMode, AddonSpec, CustomPatchAddon, Error, LaunchOptions, MetadataPatch, PatchMode,
    RuntimeOptions, addon_kit::PistonMetaGenericArgument,
};

mod common;

use common::addon::{
    build_launcher, dummy_agent_addon, jvm_flag_addon, seed_local_vanilla_instance,
    setup_fake_java_17, step_ids, test_credential, tweaker_addon,
};

#[tokio::test]
async fn scoped_patch_modes_reject_physical_addon_under_virtual_policy() {
    let root = common::temp_root("elemental-addon-scoped-reject");
    let java_home = setup_fake_java_17(&root);
    seed_local_vanilla_instance(&root, "scoped-inst");

    let launcher = build_launcher(&root, &java_home)
        .register_addon(tweaker_addon(
            "tweaker-pp",
            "Tweaker PP",
            10,
            "com.mumfrey.liteloader.launch.LiteLoaderTweaker",
        ))
        .unwrap();

    let err = launcher
        .scope()
        .patch_mode(PatchMode::Virtual)
        .addon(AddonSpec::new("tweaker-pp").with_patch_mode(AddonPatchMode::Physical))
        .install_addons("scoped-inst")
        .await
        .err()
        .expect("virtual instance must forbid physical addon mode");
    assert!(matches!(
        err,
        Error::Addon(elemental_launcher::addon_kit::Error::PatchModeDisallowedByInstance { .. })
    ));

    common::remove(&root);
}

#[tokio::test]
async fn scoped_patch_modes_enforce_monotonic_rules_and_baseline_probe() {
    let root = common::temp_root("elemental-addon-scoped");
    let java_home = setup_fake_java_17(&root);
    let json_path = seed_local_vanilla_instance(&root, "scoped-inst");

    let (agent_jar, agent_addon) =
        dummy_agent_addon(&root, "agent.jar", "acme-agent", "Acme Agent", "mode=test");
    let debug_addon = jvm_flag_addon("debug-flag", "Debug Flag", 20, "-Ddebug.only=true");
    // Mimic a physical patch carrying com.mumfrey:liteloader to prove baseline probe avoids mis-detection.
    let tweaker_pp = tweaker_addon(
        "tweaker-pp",
        "Tweaker PP",
        10,
        "com.mumfrey.liteloader.launch.LiteLoaderTweaker",
    );

    let launcher = build_launcher(&root, &java_home)
        .register_addon(tweaker_pp)
        .unwrap()
        .register_addon(debug_addon)
        .unwrap()
        .register_addon(agent_addon)
        .unwrap();

    let prepared = launcher
        .scope()
        .patch_mode(PatchMode::Physical)
        .addon(AddonSpec::new("tweaker-pp"))
        .addon(AddonSpec::new("debug-flag").with_patch_mode(AddonPatchMode::Virtual))
        .addon(AddonSpec::new("acme-agent"))
        .install_addons("scoped-inst")
        .await
        .unwrap();

    let modes: Vec<PatchMode> = prepared
        .addons()
        .iter()
        .map(|a| a.effective_patch_mode)
        .collect();
    assert_eq!(
        modes,
        vec![PatchMode::Physical, PatchMode::Virtual, PatchMode::Virtual]
    );

    let disk_json = fs::read_to_string(&json_path).unwrap();
    assert!(disk_json.contains("net.minecraft.launchwrapper.Launch"));
    assert!(!disk_json.contains("-Ddebug.only=true"));

    let credential = test_credential().await;
    let cmd = launcher
        .build_launch_command(
            &prepared,
            &credential,
            &RuntimeOptions::default(),
            &LaunchOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(cmd.main_class, "net.minecraft.launchwrapper.Launch");
    assert!(cmd.jvm_arguments.contains(&"-Ddebug.only=true".to_owned()));
    assert!(
        cmd.jvm_arguments
            .contains(&format!("-javaagent:{}=mode=test", agent_jar.display()))
    );

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

    let addon_a = tweaker_addon("addon-a", "Addon A", 0, "example.TweakerA");
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
        .scope()
        .addon(AddonSpec::new("addon-a"))
        .addon(AddonSpec::new("addon-b"))
        .install_addons("trace-inst")
        .await
        .unwrap();

    let trace = launcher.trace_instance_patches("trace-inst").await.unwrap();
    assert_eq!(step_ids(&trace), vec!["addon-a", "addon-b"]);
    assert_eq!(
        trace.steps[0].main_class,
        Some((
            "net.minecraft.client.main.Main".to_owned(),
            "net.minecraft.launchwrapper.Launch".to_owned(),
        ))
    );
    assert_eq!(trace.steps[1].main_class, None);
    assert_eq!(trace.steps[1].java_major_version, Some((17, 21)));

    let reordered = launcher
        .reorder_addons("trace-inst", &["addon-b", "addon-a"])
        .await
        .unwrap();
    assert_eq!(step_ids(&reordered), vec!["addon-b", "addon-a"]);
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
