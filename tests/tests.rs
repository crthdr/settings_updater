use std::path::PathBuf;

use indexmap::IndexMap;
use settings_updater::xml::*;
use settings_updater::*;

fn file(path: &str) -> String {
    read_file_wtf(path).expect(path)
}

#[test]
fn test1() {
    let docs_w3 = "tests/test1/Documents/The Witcher 3";
    let game_dir = "tests/test1";
    let mods = make_mods(game_dir).unwrap();

    let temp = "shit/test1.settings";
    let temp2 = "shit/test1_settings.settings";
    let temp2_dx12 = "shit/test1_settings_dx12.settings";
    let temp3 = "shit/test1.xml";
    let temp4 = "shit/test1_state.xml";

    process_inputs(&mods, &docs_w3, Some(&PathBuf::from(temp))).unwrap();
    process_settings(&mods, &docs_w3, false, Some(&PathBuf::from(temp2))).unwrap();
    process_settings(&mods, &docs_w3, true, Some(&PathBuf::from(temp2_dx12))).unwrap();
    process_xml(
        &mods,
        &game_dir,
        Some(&PathBuf::from(temp3)),
        Some(&PathBuf::from(temp4)),
    )
    .unwrap();

    assert_eq!(file(temp), file("tests/test1/out/input.settings"));
    assert_eq!(file(temp2), file("tests/test1/out/user.settings"));
    assert_eq!(file(temp2_dx12), file("tests/test1/out/dx12user.settings"));
    assert_eq!(file(temp3), file("tests/test1/out/input.xml"));
    assert_eq!(
        file(temp4),
        file("tests/test1/out/input.settings_updater.txt")
    );
}

#[test]
fn gameconf() {
    let content = file("tests/test2/gameconf.cfg");
    let w3 = parse_gameconf(&content).unwrap();
    assert_eq!(w3, "The Witcher 3 Test");
}

#[test]
fn xml() {
    let entries = xml_parse("tests/test3/input.xml").unwrap();
    println!("{:#?}", entries);
    assert_eq!(entries.len(), 65);
}

#[test]
fn xml_apply() {
    let mut e: IndexMap<String, String> = Default::default();
    e.insert("builder".into(), "Input".into());
    e.insert("displayName".into(), "NEW".into());

    let diff = vec![e];

    let temp = "shit/test4.xml";

    apply_xml_diff(
        "tests/test4/input.xml",
        &diff,
        &vec![],
        Some(&PathBuf::from(temp)),
    )
    .unwrap();
    assert_eq!(file(temp), file("tests/test4/out/input.xml"))
}

#[test]
fn settings() {
    let docs_w3 = "tests/test1/Documents/The Witcher 3";
    let mods = make_mods("tests/test1").unwrap();

    let temp = "shit/test1_settings.settings";

    process_settings(&mods, &docs_w3, false, Some(&PathBuf::from(temp))).unwrap();

    assert_eq!(file(temp), file("tests/test1/out/user.settings"))
}

#[test]
fn test5() {
    let game_dir = "tests/test5";
    let mods = make_mods(game_dir).unwrap();

    let temp3 = "shit/test5.xml";
    let temp4 = "shit/test5_state.xml";

    process_xml(
        &mods,
        &game_dir,
        Some(&PathBuf::from(temp3)),
        Some(&PathBuf::from(temp4)),
    )
    .unwrap();

    assert_eq!(file(temp3), file("tests/test5/out/input.xml"));
    assert_eq!(file(temp4), file("tests/test5/out/state.xml"));
}

#[test]
fn test6() {
    // utf-16

    let game_dir = "tests/test6";
    let mods = make_mods(game_dir).unwrap();

    let temp3 = "shit/test6.xml";
    let temp4 = "shit/test6_state.xml";

    process_xml(
        &mods,
        &game_dir,
        Some(&PathBuf::from(temp3)),
        Some(&PathBuf::from(temp4)),
    )
    .unwrap();

    assert_eq!(file(temp3), file("tests/test6/expect/input.xml"));
    assert_eq!(file(temp4), file("tests/test6/expect/state.xml"));
}

#[test]
fn test7() {
    let docs_w3 = "tests/test7/Documents/The Witcher 3";
    let game_dir = "tests/test7";
    let mods = make_mods(game_dir).unwrap();
    let temp = "shit/test7.settings";
    
    process_inputs(&mods, &docs_w3, Some(&PathBuf::from(temp))).unwrap();

    assert_eq!(file(temp), file("tests/test7/expect/input.settings"));
}
