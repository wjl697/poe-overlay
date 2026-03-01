use encoding_rs::{GB18030, GBK, UTF_8};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub chapter: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub notes: String,
    pub steps: Vec<Step>,
}

fn generate_stable_id(chapter: &str, text: &str, index: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}|{}|{}", chapter, text, index).as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ParsedDocument, String> {
    let bytes = fs::read(path).map_err(|e| format!("无法读取文件: {}", e))?;

    // 多编码兼容读取: 依次尝试 UTF-8, GBK, GB18030
    let content = if let Some(s) = UTF_8.decode_without_bom_handling_and_without_replacement(&bytes) {
        s.into_owned()
    } else if let Some(s) = GBK.decode_without_bom_handling_and_without_replacement(&bytes) {
        s.into_owned()
    } else {
        let (cow, _, _) = GB18030.decode(&bytes);
        cow.into_owned()
    };

    let mut notes = String::new();
    let mut steps_section = String::new();

    let mut in_notes = false;
    let mut in_steps = false;

    for line in content.lines() {
        let line = line.trim();
        if line.contains("注释区：") {
            in_notes = true;
            in_steps = false;
            continue;
        } else if line.contains("剧情区：") {
            in_notes = false;
            in_steps = true;
            continue;
        }

        if in_notes {
            notes.push_str(line);
            notes.push('\n');
        } else if in_steps {
            steps_section.push_str(line);
            steps_section.push('\n');
        }
    }

    let notes = notes.trim().to_string();

    let mut steps = Vec::new();
    let title_regex = Regex::new(r"^#\s+(.+)$").unwrap();
    let mut current_chapter = String::from("默认章节");
    let mut step_index = 0;

    for line in steps_section.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(caps) = title_regex.captures(line) {
            current_chapter = caps.get(1).unwrap().as_str().to_string();
            step_index = 0; // 重置章节内序号
        } else {
            let id = generate_stable_id(&current_chapter, line, step_index);
            steps.push(Step {
                id,
                chapter: current_chapter.clone(),
                text: line.to_string(),
            });
            step_index += 1;
        }
    }

    Ok(ParsedDocument { notes, steps })
}
