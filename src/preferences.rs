use crate::download::{Content, FormatOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub contents: Vec<Content>,
    pub format: FormatOptions,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub dark: bool,
    pub language: String,
    pub output: String,
    pub cookies: String,
    pub retries: u8,
    pub recent_paths: Vec<String>,
    pub presets: Vec<Preset>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            dark: true,
            language: "zh-TW".into(),
            output: String::new(),
            cookies: String::new(),
            retries: 3,
            recent_paths: Vec::new(),
            presets: Vec::new(),
        }
    }
}
impl Preferences {
    pub fn path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| ".".into());
        #[cfg(target_os = "macos")]
        let base = home.join("Library/Application Support");
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or(home);
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".config"));
        base.join("Flowmoor-rust/settings.json")
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let settings: Self =
                    serde_json::from_slice(&bytes).map_err(|e| format!("設定檔無法讀取：{e}"))?;
                if !matches!(settings.language.as_str(), "zh-TW" | "en")
                    || settings.retries > 10
                    || settings.presets.iter().any(|p| {
                        p.name.trim().is_empty()
                            || p.contents.is_empty()
                            || p.format.max_height.is_some_and(|h| h == 0 || h > 16384)
                    })
                {
                    return Err("設定檔包含無效下載選項".into());
                }
                Ok(settings)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                retries: 3,
                ..Self::default()
            }),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(path.parent().ok_or("設定檔路徑無效")?)
            .map_err(|e| e.to_string())?;
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, data).map_err(|e| e.to_string())?;
        std::fs::rename(temp, path).map_err(|e| format!("無法保存設定：{e}"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip_and_invalid_values() {
        let path = std::env::temp_dir().join(format!("flowmoor-prefs-{}.json", std::process::id()));
        let p = Preferences {
            dark: false,
            language: "en".into(),
            presets: vec![Preset {
                name: "測試".into(),
                contents: vec![Content::Audio],
                format: FormatOptions::default(),
            }],
            ..Default::default()
        };
        p.save(&path).unwrap();
        assert!(!Preferences::load(&path).unwrap().dark);
        assert_eq!(Preferences::load(&path).unwrap().language, "en");
        assert_eq!(
            Preferences::load(&path).unwrap().presets[0].contents,
            vec![Content::Audio]
        );
        std::fs::write(&path, br#"{"retries":255}"#).unwrap();
        assert!(Preferences::load(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
