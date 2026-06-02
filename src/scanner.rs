#![allow(dead_code)]
use std::process::{Command, Stdio};
use std::io::Read;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ScannerElement {
    pub id: u32,
    #[serde(rename = "type")]
    pub element_type: String,
    pub text: String,
    pub monitor_index: u32,
    pub center: [i32; 2],
    pub bbox: [i32; 4],
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ScannerOutput {
    pub screen_width: i32,
    pub screen_height: i32,
    pub elements: Vec<ScannerElement>,
}

pub fn run_visual_scan() -> anyhow::Result<ScannerOutput> {
    let mut child = Command::new("waywarp-scanner")
        .arg("scan")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "waywarp-scanner not found in PATH.\n\
                     Please install it via: uv tool install waywarp-scanner"
                )
            } else {
                anyhow::anyhow!("Failed to spawn waywarp-scanner: {:?}", e)
            }
        })?;

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);
    let mut status = None;

    while start.elapsed() < timeout {
        if let Some(s) = child.try_wait()? {
            status = Some(s);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    if status.is_none() {
        let _ = child.kill();
        return Err(anyhow::anyhow!("waywarp-scanner visual scan timed out after 10 seconds."));
    }

    let exit_status = status.unwrap();
    if !exit_status.success() {
        let mut err_msg = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut err_msg);
        }
        return Err(anyhow::anyhow!(
            "waywarp-scanner exited with non-zero code {}.\nError: {}",
            exit_status.code().unwrap_or(-1),
            err_msg.trim()
        ));
    }

    let mut json_str = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout.read_to_string(&mut json_str)?;
    }

    let parsed: ScannerOutput = serde_json::from_str(&json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse scanner output JSON: {:?}. Output was: {}", e, json_str))?;

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_json_deserialization() {
        let raw_json = r#"{
            "screen_width": 1920,
            "screen_height": 1080,
            "elements": [
                {
                    "id": 0,
                    "type": "button",
                    "text": "Login",
                    "monitor_index": 0,
                    "center": [100, 200],
                    "bbox": [80, 180, 40, 40]
                }
            ]
        }"#;
        let parsed: ScannerOutput = serde_json::from_str(raw_json).unwrap();
        assert_eq!(parsed.screen_width, 1920);
        assert_eq!(parsed.elements.len(), 1);
        assert_eq!(parsed.elements[0].text, "Login");
        assert_eq!(parsed.elements[0].center, [100, 200]);
    }
}
