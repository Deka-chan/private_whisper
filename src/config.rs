use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Cuda,
    Cpu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub hotkey: String,
    pub execution_provider: Provider,
    pub input_device: Option<String>,
    pub paste_delay_ms: u64,
    pub model_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Space".to_string(),
            execution_provider: Provider::Cuda,
            input_device: None,
            paste_delay_ms: 80,
            model_dir: None,
        }
    }
}

use std::path::Path;
use directories::ProjectDirs;

impl Config {
    /// Standard config file location: %APPDATA%/privatewhisper/config.toml (or platform equiv).
    pub fn default_path() -> anyhow::Result<PathBuf> {
        let pd = ProjectDirs::from("", "", "privatewhisper")
            .ok_or_else(|| anyhow::anyhow!("cannot determine config dir"))?;
        Ok(pd.config_dir().join("config.toml"))
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(toml::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Load from the default path, creating it with defaults if absent.
    pub fn load_or_create() -> anyhow::Result<Config> {
        let path = Self::default_path()?;
        let cfg = Self::load_from(&path)?;
        if !path.exists() {
            cfg.save_to(&path)?;
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_ctrl_space_and_cuda() {
        let c = Config::default();
        assert_eq!(c.hotkey, "Ctrl+Space");
        assert_eq!(c.execution_provider, Provider::Cuda);
        assert_eq!(c.paste_delay_ms, 80);
    }

    #[test]
    fn toml_round_trip_preserves_values() {
        let c = Config {
            hotkey: "Ctrl+Alt+D".into(),
            execution_provider: Provider::Cpu,
            input_device: Some("Mic".into()),
            paste_delay_ms: 120,
            model_dir: None,
        };
        let s = toml::to_string(&c).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let back: Config = toml::from_str("hotkey = \"Ctrl+Space\"\n").unwrap();
        assert_eq!(back.execution_provider, Provider::Cuda); // from #[serde(default)]
        assert_eq!(back.paste_delay_ms, 80);
    }

    #[test]
    fn save_then_load_from_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let c = Config { paste_delay_ms: 200, ..Default::default() };
        c.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.paste_delay_ms, 200);
    }

    #[test]
    fn load_from_missing_path_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.toml");
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded, Config::default());
    }
}
