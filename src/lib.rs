use std::collections::{HashMap, HashSet};
use std::fs::{read_dir, read_to_string, write};
use std::path::{Path, PathBuf};
pub type Res<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use crate::xml::*;
use dirs_next::document_dir;
use regex::Regex;

pub mod xml;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Section {
    name: String,
    entries: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Entry {
    left: String,
    right: String,
    action: String,
    extra: String,
    pad: bool,
    keyboard: bool,
}

fn finish(section_now: &mut Option<Section>, entries: &mut Vec<Entry>, ini: &mut Ini) {
    if let Some(section) = section_now {
        section.entries.extend(entries.drain(..));

        match ini.iter_mut().find(|s| s.name == section.name) {
            Some(existing) => {
                existing.entries.extend(section.entries.drain(..));
            }
            None => {
                ini.push(section.clone());
            }
        }
    }
}

fn start(line: &str, section_now: &mut Option<Section>) {
    let section_name = line
        .replace("[", "")
        .replace("]", "")
        .trim_start()
        .trim_end()
        .to_string();
    let section = Section {
        name: section_name,
        entries: Vec::new(),
    };
    *section_now = Some(section);
}

fn line_to_entry(line: &str) -> Res<Entry> {
    let index1 = match line.find("=") {
        None => return Err("entry =".into()),
        Some(index) => index,
    };
    let left = &line[0..index1];
    let right = &line[index1 + 1..];
    let mut action: String = String::default();
    let mut extra: String = String::default();

    let mut input = false;
    if left != "Version" && right.starts_with("(") && right.ends_with(")") {
        let index2 = match right.find("=") {
            None => return Err("action =".into()),
            Some(index) => index,
        };
        // 1 for (
        let action_left = &right[1..index2];
        if action_left != "Action" {
            return Err("not Action".into());
        }

        let search_action = &right[index2 + 1..];
        let index3 = match search_action.find(&[',', ')']) {
            None => return Err("action wtf".into()),
            Some(index) => index,
        };
        action = search_action[..index3].into();
        input = true;

        if let Some(index) = right.find(",") {
            extra = right[index..right.len() - 1].into()
        }
    }
    let pad = left.starts_with("IK_Pad_") || left.starts_with("IK_PS4_");
    let entry = Entry {
        left: left.into(),
        right: right.into(),
        action: action,
        extra: extra,
        pad: input && pad,
        keyboard: input && !pad && left != "Version",
    };

    Ok(entry)
}

pub type Ini = Vec<Section>;

fn parse_settings<P: AsRef<Path>>(path: P) -> Res<Ini> {
    println!("parsing {:?}", path.as_ref());

    let content = read_to_string(path)?;

    let split = content.split_whitespace();

    let mut ini: Ini = Default::default();
    let mut section_now: Option<Section> = None;
    let mut entries: Vec<Entry> = Vec::new();

    for line in split {
        if line.starts_with("[") {
            finish(&mut section_now, &mut entries, &mut ini);
            start(&line, &mut section_now)
        } else {
            let entry = line_to_entry(&line)?;
            entries.push(entry);
        }
    }

    finish(&mut section_now, &mut entries, &mut ini);
    Ok(ini)
}

fn section_entry_whitelist(ini: &Ini) -> HashSet<String> {
    let mut result: HashSet<String> = Default::default();

    for section in ini.iter().rev() {
        for entry in section.entries.iter().rev() {
            result.insert(entry.right.clone());
        }
    }
    return result;
}

fn ini_stringify(ini: &Ini, newline: bool) -> String {
    let mut s = String::new();

    for section in ini {
        s += "[";
        s += &section.name;
        s += "]";
        s += "\r\n";

        for entry in &section.entries {
            s += &entry.left;
            s += "=";
            s += &entry.right;
            s += "\r\n";
        }
        if newline {
            s += "\r\n";
        }
    }
    return s;
}

fn make_set_keys(ini: &Ini) -> HashMap<String, String> {
    let mut result: HashMap<String, String> = Default::default();

    for section in ini.iter().rev() {
        for entry in section.entries.iter().rev() {
            if !entry.keyboard {
                continue;
            }
            if entry.left == "IK_None" {
                continue;
            }

            if let None = result.get(&entry.right) {
                result.insert(entry.right.clone(), entry.left.clone());
            }
        }
    }
    return result;
}

fn fix_input(ini: &mut Ini, whitelist: &HashSet<String>) {
    let keys = make_set_keys(ini);
    
    for section in ini.iter_mut() {
        for entry in &mut section.entries {
            if !entry.keyboard {
                continue;
            }
            if !whitelist.contains(&entry.right) {
                continue;
            }
            
            if let Some(left) = keys.get(&entry.right) {
                entry.left = left.clone();
            }
        }
    }
    
    let mut pairs: HashMap<(String, Entry), i32> = Default::default();
    for section in ini.iter() {
        for entry in section.entries.iter() {
            let key = (section.name.clone(), entry.clone());
            let mut c: i32 = pairs.get(&key).copied().unwrap_or(0);
            c += 1;

            pairs.insert(key.clone(), c);
        }
    }

    for section in ini.iter_mut() {
        section.entries.retain(|entry| {
            let key = (section.name.clone(), entry.clone());
            if let Some(c) = pairs.get_mut(&key) {
                if *c > 1 {
                    *c -= 1;
                    return false;
                }
            }
            return true;
        });
    }

    let mut section_counts: HashMap<String, i32> = Default::default();
    for section in ini.iter() {
        let mut c = section_counts.get_mut(&section.name).copied().unwrap_or(0);
        c += 1;
        section_counts.insert(section.name.clone(), c);
    }

    ini.retain(|section| { 
        let c = section_counts.get(&section.name).copied().unwrap_or(0);
        if c == 1 {
            return true
        }
        if section.entries.len() > 0 {
            return true
        }
        return false
     });
}

fn ini_add(first: &mut Ini, second: &Ini) {
    for section in second {
        first.push(section.clone())
    }
}

#[derive(Debug)]
pub struct Mod {
    name: String,
    path: PathBuf,
    xmls: Vec<PathBuf>,
    inputs: Vec<PathBuf>,
    settings: Vec<PathBuf>,
}

fn make_mod(root: &Path) -> Res<Mod> {
    let mut m = Mod {
        name: path_to_file_name(&root)?,
        path: root.into(),
        xmls: vec![
            root.join("bin/config/r4game/user_config_matrix/pc/input.xml"),
            root.join("input.xml.txt"),
            root.join("input.xml.part.txt"),
        ],
        inputs: vec![
            root.join("input.settings.txt"),
            root.join("input.settings.part.txt"),
        ],
        settings: vec![
            root.join("user.settings.txt"),
            root.join("dx12user.settings.txt"),
            root.join("user.settings.part.txt"),
            root.join("dx12user.settings.part.txt"),
        ],
    };

    m.xmls.retain(|file| file.exists());
    m.inputs.retain(|file| file.exists());
    m.settings.retain(|file| file.exists());

    Ok(m)
}

fn path_to_file_name(path: &Path) -> Res<String> {
    let s = path.file_name()
        .ok_or("file name wtf".to_string())?
        .to_str()
        .ok_or("file name wtf".to_string())?;
    Ok(s.to_string())
}

pub fn make_mods<P>(game_dir: P) -> Res<Vec<Mod>>
where
    P: AsRef<Path>,
{
    let mut mods: Vec<Mod> = Vec::new();

    for entry in read_dir(game_dir.as_ref().join("mods"))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let file_name = path_to_file_name(&entry.path())?;
        if !file_name.starts_with("mod") {
            continue;
        }
        if let Ok(m) = make_mod(&entry.path()) {
            mods.push(m);
        }
    }
    Ok(mods)
}

pub fn process_inputs<P1>(mods: &[Mod], docs_w3: &P1, input_out_test: Option<&Path>) -> Res<()>
where
    P1: AsRef<Path>,
{
    let docs_input_path = docs_w3.as_ref().join("input.settings");
    if !docs_input_path.exists() {
        return Ok(());
    }

    let mut the_input: Ini = Ini::default();
    for m in mods {
        for input in &m.inputs {
            if let Ok(ini) = parse_settings(input) {
                ini_add(&mut the_input, &ini);
            }
        }
    }

    let white = section_entry_whitelist(&the_input);

    let docs_input = parse_settings(&docs_input_path)?;
    ini_add(&mut the_input, &docs_input);

    fix_input(&mut the_input, &white);

    let content = ini_stringify(&the_input, true);

    let out = match input_out_test {
        Some(input_out) => input_out.to_path_buf(),
        None => docs_input_path,
    };
    write(out, content)?;

    Ok(())
}

fn filename_wtf(path: &Path) -> &str {
    match path.file_name() {
        None => "",
        Some(s) => match s.to_str() {
            None => "",
            Some(s) => s,
        },
    }
}

pub fn process_settings<P>(
    mods: &[Mod],
    docs_w3: &P,
    dx12: bool,
    settings_out_test: Option<&Path>,
) -> Res<()>
where
    P: AsRef<Path>,
{
    let stem = if dx12 { "dx12user" } else { "user" };
    let name = format!("{}{}", stem, ".settings");
    let docs_settings_path = docs_w3.as_ref().join(name);
    if !docs_settings_path.exists() {
        return Ok(());
    }

    let mut the_settings: Ini = Ini::default();
    let mut files: Vec<PathBuf> = Default::default();
    for m in mods {
        let has_dx12 = m.settings.iter().any(|s| filename_wtf(s).contains("dx12"));
        for sett in &m.settings {
            if ((filename_wtf(sett).contains("dx12") && dx12) || !has_dx12)
                || (!filename_wtf(sett).contains("dx12") && !dx12)
            {
                files.push(sett.clone());
            }
        }
    }
    for file in files {
        if let Ok(ini) = parse_settings(file) {
            ini_add(&mut the_settings, &ini);
        }
    }

    let docs_settings = parse_settings(&docs_settings_path)?;
    ini_add(&mut the_settings, &docs_settings);

    let content = ini_stringify(&the_settings, false);

    let out = match settings_out_test {
        Some(input_out) => input_out.to_path_buf(),
        None => docs_settings_path,
    };
    write(out, content)?;

    Ok(())
}

fn get_game_dir() -> Res<PathBuf> {
    let exe = PathBuf::from("witcher3.exe");

    if !exe.exists() {
        return Err("no exe".into());
    }

    let path = PathBuf::from("../..");

    Ok(path)
}

pub fn parse_gameconf(content: &str) -> Res<String> {
    let re = Regex::new(r#"title\s+"(.*)""#)?;
    if let Some(caps) = re.captures(content) {
        let title = caps[1].to_string();
        Ok(title)
    } else {
        return Err("gameconf failed".into());
    }
}

fn gameconf_w3(game_dir: &Path) -> Res<String> {
    let path = game_dir.join("bin/gameconf.cfg");
    let content = read_to_string(path)?;
    let title = parse_gameconf(&content)?;
    Ok(title)
}

const INPUT_XML_ORIG: &str = include_str!("./input.xml");

pub fn process_xml<P>(
    mods: &[Mod],
    game_dir: P,
    out_path_test: Option<&Path>,
    out_state_test: Option<&Path>,
) -> Res<()>
where
    P: AsRef<Path>,
{
    let mut entries: Vec<XmlEntry> = Default::default();

    for m in mods {
        for x in &m.xmls {
            if let Ok(parsed) = xml_parse(&x) {
                xml_add(&mut entries, &parsed);
            }
        }
    }

    let bin_pc_dir = game_dir
        .as_ref()
        .join("bin/config/r4game/user_config_matrix/pc");
    
    let bin_state_path = bin_pc_dir.join("input.settings_updater.txt");

    let mut to_delete: Vec<XmlEntry> = Vec::default();

    if bin_state_path.exists() {
        let mut state = xml_parse(&bin_state_path)?;

        for entry in state.iter_mut() {
            if !entries.contains(&entry) {
                to_delete.push(entry.clone());
            }
        }
    }
    
    let bin_input_path = bin_pc_dir.join("input.xml");

    let orig = xml_parse_content(&INPUT_XML_ORIG)?;
    let to_add = compare_xml(&orig, &entries)?;

    apply_xml_diff(&bin_input_path, &to_add, &to_delete, out_path_test)?;

    xml_save(&bin_state_path, &to_add, &to_delete, out_state_test)?;

    Ok(())
}

pub fn process_all() -> Res<()> {
    let docs_dir = document_dir().ok_or("no documents".to_string())?;
    let game_dir = get_game_dir()?;
    let w3 = gameconf_w3(&game_dir)?;

    let mods = make_mods(&game_dir)?;

    let docs_w3 = docs_dir.join(w3);

    let _ = process_inputs(&mods, &docs_w3, None);
    let _ = process_settings(&mods, &docs_w3, false, None);
    let _ = process_settings(&mods, &docs_w3, true, None);
    let _ = process_xml(&mods, &game_dir, None, None);

    Ok(())
}

#[cfg(not(debug_assertions))]
#[ctor::ctor]
fn asi_main() {
    let _ = process_all();
}
