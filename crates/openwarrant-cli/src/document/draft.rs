// SPDX-License-Identifier: Apache-2.0
//! Offline interactive authoring. Checkpoints are new files; publication never clobbers.
use camino::Utf8Path;
use openwarrant_core::document as d;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};
const STATE_LIMIT: usize = 1024 * 1024;
const SESSION_LIMIT: u64 = 16 * 1024 * 1024;
const LINE_LIMIT: usize = 64 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    id: String,
    kind: String,
    text: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    schema: String,
    metadata: Vec<(String, Value)>,
    units: Vec<Unit>,
}
impl Draft {
    fn new() -> Self {
        Self {
            schema: "oh.war/interactive-draft/v1".into(),
            metadata: vec![
                ("schema".into(), json!("oh.war/document/1.0.0-rc.3")),
                ("kind".into(), json!("warrant")),
                ("id".into(), json!("draft:untitled")),
                ("revision".into(), json!(1)),
                ("title".into(), json!("Untitled Warrant")),
                ("state".into(), json!("draft")),
            ],
            units: vec![
                Unit {
                    id: "outcome".into(),
                    kind: "binding".into(),
                    text: "# Untitled Warrant\n\nDescribe the required outcome.\n".into(),
                },
                Unit {
                    id: "scope".into(),
                    kind: "binding".into(),
                    text: "## Scope\n\nDefine the work boundary.\n".into(),
                },
                Unit {
                    id: "context".into(),
                    kind: "binding".into(),
                    text: "## Context\n\nRecord relevant sources and unknowns.\n".into(),
                },
            ],
        }
    }
    fn source(&self) -> Result<Vec<u8>, String> {
        let fields = d::DocumentFields {
            metadata: self
                .metadata
                .iter()
                .map(|(key, value)| {
                    Ok((
                        key.clone(),
                        serde_json::from_value(value.clone())
                            .map_err(|e| format!("metadata: {e}"))?,
                    ))
                })
                .collect::<Result<_, String>>()?,
            units: self
                .units
                .iter()
                .map(|unit| {
                    Ok(d::AuthoredUnit {
                        id: unit.id.clone(),
                        kind: match unit.kind.as_str() {
                            "binding" => d::UnitKind::Binding,
                            "background" => d::UnitKind::Background,
                            _ => return Err("Unknown unit kind".into()),
                        },
                        text: unit.text.clone(),
                    })
                })
                .collect::<Result<_, String>>()?,
        };
        d::author_document(
            &fields,
            d::AuthorOptions::default(),
            d::ParseLimits::default(),
        )
        .map_err(|e| format!("{}: {}", e.code, e.message))
    }
    fn set(&mut self, key: &str, value: Value) {
        if key == "title"
            && let Some(title) = value.as_str()
            && let Some(unit) = self.units.iter_mut().find(|unit| unit.id == "outcome")
        {
            let rest = unit.text.split_once('\n').map(|(_, r)| r).unwrap_or("");
            unit.text = format!("# {title}\n{rest}");
        }
        if let Some(field) = self.metadata.iter_mut().find(|(k, _)| k == key) {
            field.1 = value;
        } else {
            self.metadata.push((key.into(), value));
        }
    }
}
struct Session {
    root: PathBuf,
    next: u32,
    bytes: u64,
    _lock: fs::File,
}
fn new_file(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temp = parent.join(format!(
        ".ow-draft-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        fs::hard_link(&temp, path)
    })();
    drop(file);
    let _ = fs::remove_file(temp);
    result
}
impl Session {
    fn open(path: &Path, resume: bool) -> Result<(Self, Draft), String> {
        if !resume {
            fs::create_dir(path).map_err(|e| format!("New draft directory: {e}"))?;
        }
        if !fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_dir()
        {
            return Err("Draft directory must be a real directory".into());
        }
        let root = path.canonicalize().map_err(|e| e.to_string())?;
        let lock_path = root.join("session.lock");
        if lock_path
            .symlink_metadata()
            .is_ok_and(|m| !m.file_type().is_file())
        {
            return Err("Invalid session lock".into());
        }
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|_| "Draft session already has a writer".to_string())?;
        let mut snapshots = Vec::new();
        let mut bytes = 0u64;
        for (count, entry) in fs::read_dir(&root).map_err(|e| e.to_string())?.enumerate() {
            if count >= 8192 {
                return Err("Draft directory entry limit exceeded".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "session.lock" || name.starts_with(".ow-draft-") {
                continue;
            }
            let ordinal = name
                .strip_prefix("draft-")
                .and_then(|s| s.strip_suffix(".json"))
                .filter(|s| s.len() == 6 && s.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or("Unknown file in draft directory")?;
            let meta = entry.path().symlink_metadata().map_err(|e| e.to_string())?;
            if !meta.file_type().is_file() || meta.len() > STATE_LIMIT as u64 {
                return Err("Invalid draft snapshot".into());
            }
            bytes = bytes.checked_add(meta.len()).ok_or("Draft size overflow")?;
            if bytes > SESSION_LIMIT || snapshots.len() >= 4096 {
                return Err("Draft session limit exceeded".into());
            }
            snapshots.push((ordinal, entry.path()));
        }
        if bytes > SESSION_LIMIT || snapshots.len() > 4096 {
            return Err("Draft session limit exceeded".into());
        }
        snapshots.sort();
        let (next, draft) = if resume {
            let (n, last) = snapshots.last().ok_or("No saved draft to resume")?;
            let mut data = Vec::new();
            fs::File::open(last)
                .map_err(|e| e.to_string())?
                .take(STATE_LIMIT as u64 + 1)
                .read_to_end(&mut data)
                .map_err(|e| e.to_string())?;
            if data.len() > STATE_LIMIT {
                return Err("Draft snapshot limit exceeded".into());
            }
            let raw = crate::sdk::wire::decode_value(&data)
                .map_err(|e| format!("Invalid saved draft: {e}"))?;
            let draft: Draft = serde_json::from_value(raw.clone())
                .map_err(|e| format!("Invalid saved draft: {e}"))?;
            crate::sdk::wire::shape(
                &raw,
                &serde_json::to_value(&draft).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Invalid saved draft: {e}"))?;
            if draft.schema != "oh.war/interactive-draft/v1" {
                return Err("Unsupported draft schema".into());
            }
            (n + 1, draft)
        } else {
            (0, Draft::new())
        };
        let mut session = Self {
            root,
            next,
            bytes,
            _lock: lock,
        };
        if !resume {
            session.checkpoint(&draft)?;
        }
        Ok((session, draft))
    }
    fn checkpoint(&mut self, draft: &Draft) -> Result<(), String> {
        let bytes = serde_json::to_vec(draft).map_err(|e| e.to_string())?;
        // The entire wrapper must be admissible to the resume decoder too.
        crate::sdk::wire::decode_value(&bytes)
            .map_err(|e| format!("Checkpoint refused; prior draft retained: {e}"))?;

        if bytes.len() > STATE_LIMIT
            || self.bytes + bytes.len() as u64 > SESSION_LIMIT
            || self.next >= 4096
        {
            return Err("Draft limit reached; prior checkpoint retained".into());
        }
        new_file(
            &self.root.join(format!("draft-{:06}.json", self.next)),
            &bytes,
        )
        .map_err(|e| e.to_string())?;
        self.next += 1;
        self.bytes += bytes.len() as u64;
        Ok(())
    }
}
fn line(input: &mut impl BufRead) -> Result<Option<String>, String> {
    let mut bytes = Vec::new();
    input
        .take(LINE_LIMIT as u64 + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > LINE_LIMIT {
        return Err("Input line limit exceeded; prior draft retained".into());
    }
    let text = String::from_utf8(bytes).map_err(|_| "Input must be UTF-8".to_string())?;
    Ok(Some(text.trim_end_matches(['\r', '\n']).to_string()))
}
const HELP: &str = "Fields: title, id, kind, revision; meta KEY (JSON value).\nUnits: unit ID binding|background (Markdown; finish with a single dot), drop ID.\nActions: preview, save, cancel, help. Every accepted line checkpoints your draft. No approval or verification.\n";
fn interact(
    input: &mut impl BufRead,
    prompts: &mut impl Write,
    session: &mut Session,
    draft: &mut Draft,
    output: &Path,
) -> Result<bool, String> {
    prompts
        .write_all(HELP.as_bytes())
        .map_err(|e| e.to_string())?;
    loop {
        write!(prompts, "draft> ")
            .and_then(|_| prompts.flush())
            .map_err(|e| e.to_string())?;
        let Some(command) = line(input)? else {
            return Ok(false);
        };
        let words: Vec<_> = command.split_whitespace().collect();
        match words.as_slice() {
            ["cancel"] => return Ok(false),
            ["help"] => prompts
                .write_all(HELP.as_bytes())
                .map_err(|e| e.to_string())?,
            ["preview"] | ["save"] => match draft.source() {
                Ok(source) => {
                    if words[0] == "save" {
                        new_file(output, &source)
                            .map_err(|e| format!("Output unchanged; draft retained: {e}"))?;
                        return Ok(true);
                    }
                    prompts.write_all(&source).map_err(|e| e.to_string())?;
                }
                Err(error) => {
                    writeln!(prompts, "Invalid draft: {error}").map_err(|e| e.to_string())?
                }
            },
            [key @ ("title" | "id" | "kind" | "revision")] | ["meta", key] => {
                writeln!(prompts, "{key} value:").map_err(|e| e.to_string())?;
                let Some(text) = line(input)? else {
                    return Ok(false);
                };
                let value = if words[0] == "meta" || *key == "revision" {
                    match crate::sdk::wire::decode_value(text.as_bytes()) {
                        Ok(value) => value,
                        Err(error) => {
                            writeln!(prompts, "Invalid JSON: {error}")
                                .map_err(|e| e.to_string())?;
                            continue;
                        }
                    }
                } else {
                    json!(text)
                };
                draft.set(key, value);
                session.checkpoint(draft)?;
            }
            ["unit", id, kind @ ("binding" | "background")] => {
                let index = if let Some(i) = draft.units.iter().position(|u| u.id == *id) {
                    i
                } else {
                    draft.units.push(Unit {
                        id: (*id).into(),
                        kind: (*kind).into(),
                        text: String::new(),
                    });
                    draft.units.len() - 1
                };
                draft.units[index].kind = (*kind).into();
                draft.units[index].text.clear();
                session.checkpoint(draft)?;
                writeln!(
                    prompts,
                    "Markdown for {id}; include heading. Single dot ends input."
                )
                .map_err(|e| e.to_string())?;
                loop {
                    let Some(text) = line(input)? else {
                        return Ok(false);
                    };
                    if text == "." {
                        break;
                    }
                    draft.units[index].text.push_str(&text);
                    draft.units[index].text.push('\n');
                    session.checkpoint(draft)?;
                }
            }
            ["drop", id] => {
                draft.units.retain(|u| u.id != *id);
                session.checkpoint(draft)?;
            }
            _ => writeln!(prompts, "Unknown command. Use help.").map_err(|e| e.to_string())?,
        }
    }
}
pub fn run(directory: &Utf8Path, output: &Utf8Path, resume: bool, mode: crate::output::Mode) -> u8 {
    let result = (|| {
        let (mut session, mut draft) = Session::open(directory.as_std_path(), resume)?;
        let parent = output
            .as_std_path()
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if parent
            .canonicalize()
            .map_err(|e| format!("Output parent: {e}"))?
            .starts_with(&session.root)
        {
            return Err("Output must be outside the checkpoint directory; draft retained".into());
        }

        interact(
            &mut io::stdin().lock(),
            &mut io::stderr().lock(),
            &mut session,
            &mut draft,
            output.as_std_path(),
        )
    })();
    let mut report = crate::diagnostic::Report::default();
    let saved = match result {
        Ok(saved) => {
            report.push(crate::diagnostic::Diagnostic::pass(
                "document.draft",
                format!(
                    "{}; draft retained at {directory}; no qualification",
                    if saved { "Source saved" } else { "Canceled" }
                ),
            ));
            saved
        }
        Err(error) => {
            report.push(crate::diagnostic::Diagnostic::error(
                "document.draft",
                directory.as_str(),
                error,
            ));
            false
        }
    };
    crate::output::finish(
        mode,
        "document.draft",
        &report,
        Some(json!({"saved":saved,"draft_directory":directory.as_str(),
        "output":if saved {Some(output.as_str())} else {None},"qualified":false})),
    )
}
