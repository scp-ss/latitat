use crate::config::Config;

/// Baked into the binary at compile time (data/default_config.json).
pub const EMBEDDED_JSON: &str = include_str!("../default_config.json");

/// Checked in this order. First one that exists AND parses wins.
pub const SEARCH_PATHS: &[&str] = &[
    "config/data/config.json",
    "config/config.json",
    "data/config.json",
    "config.json",
];

#[derive(Debug)]
pub enum Source {
    File(String),
    Embedded,
    Defaults,
}

pub struct Loaded {
    pub config: Config,
    pub source: Source,
    pub warnings: Vec<String>,
}

/// `read` takes a relative path and returns the file text if it exists.
/// The caller decides how files are read (std::fs, Godot FileAccess, ...).
pub fn load_with(read: impl Fn(&str) -> Option<String>) -> Loaded {
    let mut warnings = Vec::new();

    for path in SEARCH_PATHS {
        if let Some(text) = read(path) {
            match Config::from_json(&text) {
                Ok(config) => {
                    return Loaded { config, source: Source::File(path.to_string()), warnings };
                }
                // File exists but is broken: warn and try the next one.
                Err(e) => warnings.push(format!("{path} is broken: {e}")),
            }
        }
    }

    match Config::from_json(EMBEDDED_JSON) {
        Ok(config) => Loaded { config, source: Source::Embedded, warnings },
        Err(e) => {
            warnings.push(format!("embedded default_config.json is broken: {e}"));
            Loaded { config: Config::default(), source: Source::Defaults, warnings }
        }
    }
}

/// Plain Rust version for tests and normal programs.
pub fn load_from_disk(root: &str) -> Loaded {
    load_with(|p| std::fs::read_to_string(format!("{root}/{p}")).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_json_matches_rust_defaults() {
        // Keeps default_config.json and defaults.rs from drifting apart.
        let embedded = Config::from_json(EMBEDDED_JSON).expect("default_config.json must parse");
        assert_eq!(embedded, Config::default());
    }

    #[test]
    fn broken_file_is_skipped() {
        let loaded = load_with(|p| (p == "config.json").then(|| "{ not json".to_string()));
        assert!(matches!(loaded.source, Source::Embedded));
        assert_eq!(loaded.warnings.len(), 1);
    }

    #[test]
    fn higher_priority_wins() {
        let loaded = load_with(|p| match p {
            "config/data/config.json" => Some(r#"{"player":{"phys":{"mass":99.0}}}"#.into()),
            "config.json" => Some(r#"{"player":{"phys":{"mass":11.0}}}"#.into()),
            _ => None,
        });
        assert_eq!(loaded.config.player.phys.mass, 99.0);
        // Missing fields fell back to defaults.
        assert_eq!(loaded.config.physics.water_density, 1000.0);
    }
}
