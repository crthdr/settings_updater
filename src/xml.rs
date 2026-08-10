use std::fs::write;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use regex::Regex;

use crate::Res;

use xml::reader::{EventReader, XmlEvent};

pub type XmlEntry = IndexMap<String, String>;

fn concat(arr: &[&str]) -> String {
    let mut s = String::new();
    for x in arr {
        s += &x;
    }
    s
}

fn xml_entry_to_string_variants(entry: &XmlEntry) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut s = String::new();

    s += "<Var";
    for (k, v) in entry {
        s += " ";
        s += &k;
        s += "=";
        s += "\"";
        s += &v;
        s += "\"";
    }

    let var1 = concat(&["\r\n", &s, "/>"]);
    let var2 = concat(&[&s, "/>"]);
    let var3 = concat(&[&s, " />"]);

    result.push(var1);
    result.push(var2);
    result.push(var3);

    result
}

pub fn compare_xml(first: &[XmlEntry], second: &[XmlEntry]) -> Res<Vec<XmlEntry>> {
    let mut result: Vec<XmlEntry> = Vec::new();

    for var in second {
        if !first.contains(var) {
            result.push(var.clone());
        }
    }
    Ok(result)
}

pub fn unapply(content: &str, entries: &Vec<XmlEntry>) -> Res<String> {
    let mut result = content.to_string();

    for entry in entries {
        let variants = xml_entry_to_string_variants(&entry);
        for var in variants {
            result = result.replace(&var, "");
        }
    }
    Ok(result)
}

pub fn apply_xml_diff<P>(
    path: P,
    to_add: &Vec<XmlEntry>,
    to_delete: &Vec<XmlEntry>,
    out_path_test: Option<&Path>,
) -> Res<()>
where
    P: AsRef<Path>,
{
    let mut content = read_file_wtf(&path)?;

    let re = Regex::new(r#"<Var.*"MoveRght".*/>"#)?;
    content = apply(&content, &to_add, &to_delete, &re)?;

    let out: PathBuf = match out_path_test {
        Some(out_path_test) => out_path_test.into(),
        None => path.as_ref().into(),
    };

    write(out, &content)?;
    Ok(())
}

pub fn apply(
    input: &str,
    to_add: &Vec<XmlEntry>,
    to_delete: &Vec<XmlEntry>,
    place: &Regex,
) -> Res<String> {
    let content: String = unapply(&input, &to_delete)?;

    let temp = xml_parse_content(&content)?;

    let mut str = String::new();
    for entry in to_add {
        if !temp.contains(&entry) {
            let s = xml_entry_to_string_variants(&entry)[0].clone();
            str += &s;
        }
    }

    let m = place.find(&content);

    let idx = match m {
        None => return Err("MoveRght not found".into()),
        Some(m) => m.end(),
    };

    let mut content2: String = content[..idx].into();
    content2 += &str;
    content2 += &content[idx..];

    Ok(content2)
}

pub fn xml_add(first: &mut Vec<XmlEntry>, second: &[XmlEntry]) {
    for x in second {
        if !first.contains(x) {
            first.push(x.clone());
        }
    }
}

const XML_TEMPLATE: &str = include_str!("./template.xml");

pub fn xml_save<P>(
    path: P,
    diff: &Vec<XmlEntry>,
    to_delete: &Vec<XmlEntry>,
    out_test: Option<&Path>,
) -> Res<()>
where
    P: AsRef<Path>,
{
    let mut content: String = if path.as_ref().exists() {
        read_file_wtf(&path)?
    } else {
        XML_TEMPLATE.into()
    };

    let re = Regex::new("<VisibleVars>")?;
    content = apply(&content, &diff, &to_delete, &re)?;

    let out: PathBuf = match out_test {
        Some(out_test) => out_test.into(),
        None => path.as_ref().into(),
    };

    std::fs::write(out, content)?;

    Ok(())
}

fn read_wtf(bytes: &[u8]) -> Res<String> {
    let mut data: Vec<u8> = bytes.into();

    if data.starts_with(&[0xFF, 0xFE]) {
        data.drain(..2);
    }
    let utf16: Vec<u16> = data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let text = String::from_utf16(&utf16)?;

    Ok(text)
}

pub fn read_file_wtf<P>(path: P) -> Res<String>
where
    P: AsRef<Path>,
{
    let data = std::fs::read(path)?;

    let content = match String::from_utf8(data.clone()) {
        Err(_) => {
            let utf16 = read_wtf(&data)?;
            utf16
        }
        Ok(utf8) => utf8,
    };
    Ok(content)
}

pub fn xml_parse<P>(path: P) -> Res<Vec<XmlEntry>>
where
    P: AsRef<Path>,
{
    let content = read_file_wtf(path)?;
    let result = xml_parse_content(&content)?;
    Ok(result)
}

pub fn xml_parse_content(content: &str) -> Res<Vec<XmlEntry>> {
    let mut result: Vec<XmlEntry> = Default::default();

    let content = content.replace(r#"encoding="UTF-16""#, r#"encoding="UTF-8""#);

    let mut cursor = Cursor::new(content);
    let parser = EventReader::new(&mut cursor);
    let mut pc_input = false;

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name == "Group" {
                    if attributes
                        .iter()
                        .any(|a| a.name.local_name == "id" && a.value == "PCInput")
                    {
                        pc_input = true;
                    }
                } else if name.local_name == "Var" {
                    if pc_input {
                        let mut entry: XmlEntry = Default::default();
                        for attr in attributes {
                            entry.insert(attr.name.local_name.into(), attr.value.into());
                        }
                        result.push(entry);
                    }
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "Group" {
                    pc_input = false;
                }
            }
            Err(err) => {
                return Err(err.into());
            }
            _ => {}
        }
    }

    Ok(result)
}
