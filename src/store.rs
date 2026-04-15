//! Minimal append only storage for hash requests.
//!
//! Requirements:
//! - Uses only Rust standard library for persistence logic.
//! - Stores one record per line (JSONL like) for easy append and inspection.
//! - Provides an ever increasing count (entry number) per file.

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct EntryStore {
    path: PathBuf,
    next_count: AtomicU64,
    write_lock: Mutex<()>,
}

impl EntryStore {
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let existing = count_lines_if_exists(&path)?;
        Ok(Self {
            path,
            next_count: AtomicU64::new(existing + 1),
            write_lock: Mutex::new(()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, text: &str, hash_hex: &str) -> io::Result<StoredEntry> {
        let _guard = self
            .write_lock
            .lock()
            .expect("entry store mutex poisoned");

        let count = self.next_count.fetch_add(1, Ordering::SeqCst);
        let timestamp_ms = now_unix_ms();
        let id = generate_id(timestamp_ms, count);

        let line = format_entry_line(&id, count, timestamp_ms, text, hash_hex);
        append_line(&self.path, &line)?;

        Ok(StoredEntry {
            id,
            count,
            timestamp_ms,
        })
    }
}

pub struct StoredEntry {
    pub id: String,
    pub count: u64,
    pub timestamp_ms: u128,
}

pub struct EntryRecord {
    pub id: String,
    pub count: u64,
    pub timestamp_ms: u128,
    pub text: String,
    pub hash: String,
}

impl EntryStore {
    pub fn read_all(&self) -> io::Result<Vec<EntryRecord>> {
        read_all_records(&self.path)
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn generate_id(timestamp_ms: u128, count: u64) -> String {
    // Not cryptographically random. For storage identity and debugging:
    // combine timestamp, pid, and monotonically increasing count.
    let pid = std::process::id();
    format!("{timestamp_ms:x}{pid:08x}{count:016x}")
}

fn append_line(path: &Path, line: &str) -> io::Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

fn count_lines_if_exists(path: &Path) -> io::Result<u64> {
    let f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let reader = BufReader::new(f);
    Ok(reader.lines().count() as u64)
}

fn format_entry_line(id: &str, count: u64, timestamp_ms: u128, text: &str, hash_hex: &str) -> String {
    // JSON without external libraries. Escape only what we may reasonably encounter.
    // One object per line:
    // {"id":"...","count":1,"timestamp_ms":...,"text":"...","hash":"..."}
    format!(
        "{{\"id\":\"{}\",\"count\":{},\"timestamp_ms\":{},\"text\":\"{}\",\"hash\":\"{}\"}}",
        json_escape(id),
        count,
        timestamp_ms,
        json_escape(text),
        json_escape(hash_hex)
    )
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                // Encode other control chars as \u00XX.
                let v = c as u32;
                out.push_str("\\u");
                out.push_str(&format!("{:04x}", v));
            }
            _ => out.push(ch),
        }
    }
    out
}

fn read_all_records(path: &Path) -> io::Result<Vec<EntryRecord>> {
    let f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let reader = BufReader::new(f);

    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if let Some(rec) = parse_record_line(&line) {
            out.push(rec);
        }
    }
    Ok(out)
}

fn parse_record_line(line: &str) -> Option<EntryRecord> {
    let id = extract_json_string_field(line, "id")?;
    let text = extract_json_string_field(line, "text")?;
    let hash = extract_json_string_field(line, "hash")?;
    let count = extract_json_u64_field(line, "count")?;
    let timestamp_ms = extract_json_u128_field(line, "timestamp_ms")?;
    Some(EntryRecord {
        id,
        count,
        timestamp_ms,
        text,
        hash,
    })
}

fn extract_json_string_field(s: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    let raw = &rest[..end];
    Some(json_unescape_minimal(raw))
}

fn extract_json_u64_field(s: &str, field: &str) -> Option<u64> {
    let needle = format!("\"{field}\":");
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find(',').or_else(|| rest.find('}'))?;
    rest[..end].trim().parse().ok()
}

fn extract_json_u128_field(s: &str, field: &str) -> Option<u128> {
    let needle = format!("\"{field}\":");
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find(',').or_else(|| rest.find('}'))?;
    rest[..end].trim().parse().ok()
}

fn json_unescape_minimal(s: &str) -> String {
    // Matches what `json_escape` emits. Not a general JSON parser.
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(ch) = it.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match it.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                // Expect 4 hex digits.
                let mut hex = String::new();
                for _ in 0..4 {
                    if let Some(h) = it.next() {
                        hex.push(h);
                    } else {
                        break;
                    }
                }
                if let Ok(v) = u32::from_str_radix(&hex, 16) {
                    if let Some(c) = char::from_u32(v) {
                        out.push(c);
                    }
                }
            }
            Some(other) => out.push(other),
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::EntryStore;
    use std::fs;

    #[test]
    fn append_writes_one_line_with_fields() {
        let mut p = std::env::temp_dir();
        let unique = format!("easyreceipt_store_test_{}_{}.jsonl", std::process::id(), super::now_unix_ms());
        p.push(unique);

        let _ = fs::remove_file(&p);
        let store = EntryStore::new(&p).expect("store init");

        let entry = store.append("hello", "deadbeef").expect("append");
        let contents = fs::read_to_string(&p).expect("read");

        assert!(contents.contains("\"id\":\""));
        assert!(contents.contains(&format!("\"count\":{}", entry.count)));
        assert!(contents.contains("\"timestamp_ms\":"));
        assert!(contents.contains("\"text\":\"hello\""));
        assert!(contents.contains("\"hash\":\"deadbeef\""));
    }

    #[test]
    fn read_all_returns_appended_records() {
        let mut p = std::env::temp_dir();
        let unique = format!(
            "easyreceipt_store_test_read_{}_{}.jsonl",
            std::process::id(),
            super::now_unix_ms()
        );
        p.push(unique);

        let _ = fs::remove_file(&p);
        let store = EntryStore::new(&p).expect("store init");
        store.append("one", "h1").expect("append 1");
        store.append("two", "h2").expect("append 2");

        let rows = store.read_all().expect("read all");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].count, 1);
        assert_eq!(rows[0].text, "one");
        assert_eq!(rows[0].hash, "h1");
        assert_eq!(rows[1].count, 2);
        assert_eq!(rows[1].text, "two");
        assert_eq!(rows[1].hash, "h2");
    }
}

