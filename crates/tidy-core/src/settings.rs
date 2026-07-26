use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::vault::Vault;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReaderSettings {
    pub theme: String,
    pub font: String,
    pub font_size: u32,
    pub line_height: f32,
    pub measure: String,
}

impl Default for ReaderSettings {
    fn default() -> Self {
        Self {
            theme: "paper".into(),
            font: "serif".into(),
            font_size: 20,
            line_height: 1.7,
            measure: "narrow".into(),
        }
    }
}

pub fn load_reader_settings(vault: &Vault) -> Result<ReaderSettings> {
    let path = vault.root().join(".tidy").join("config.toml");
    if !path.exists() {
        return Ok(ReaderSettings::default());
    }
    let text = std::fs::read_to_string(&path)?;
    Ok(parse_reader_from_toml(&text))
}

pub fn save_reader_settings(vault: &Vault, settings: &ReaderSettings) -> Result<()> {
    let path = vault.root().join(".tidy").join("config.toml");
    let contents = format!(
        "# Tidy vault configuration\nschema_version = 1\n\n[reader]\ntheme = {:?}\nfont = {:?}\nfont_size = {}\nline_height = {}\nmeasure = {:?}\n",
        settings.theme, settings.font, settings.font_size, settings.line_height, settings.measure
    );
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, contents.as_bytes())?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn parse_reader_from_toml(text: &str) -> ReaderSettings {
    let mut settings = ReaderSettings::default();
    let mut in_reader = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_reader = line == "[reader]";
            continue;
        }
        if !in_reader {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'').to_owned();
        match key {
            "theme" => settings.theme = value,
            "font" => settings.font = value,
            "font_size" => {
                if let Ok(parsed) = value.parse() {
                    settings.font_size = parsed;
                }
            }
            "line_height" => {
                if let Ok(parsed) = value.parse() {
                    settings.line_height = parsed;
                }
            }
            "measure" => settings.measure = value,
            _ => {}
        }
    }
    settings
}
