use std::fs::{self};

use std::env;

#[derive(Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    pub enable_hints: Option<bool>,
    pub enable_telemetry: Option<bool>,
}

impl GlobalSettings {
    pub fn get_settings_file_path() -> &'static str {
        "~/.clarinet/clarinetrc.toml"
    }

    pub fn from_global_file() -> Self {
        let home_dir = dirs::home_dir();

        if let Some(path) = home_dir.map(|home_dir| home_dir.join(".clarinet/clarinetrc.toml")) {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(content) => match toml::from_str::<GlobalSettings>(&content) {
                        Ok(res) => return res,
                        Err(_) => {
                            println!(
                                "{} {}",
                                format_warn!("unable to parse"),
                                Self::get_settings_file_path()
                            );
                        }
                    },
                    Err(_) => {
                        println!(
                            "{} {}",
                            format_warn!("unable to read file"),
                            Self::get_settings_file_path()
                        );
                    }
                }
            }
        };

        // Keep backwards compatibility with ENV var
        let enable_hints = match env::var("CLARINET_DISABLE_HINTS") {
            Ok(v) => Some(v == "1"),
            Err(_) => None,
        };
        Self {
            enable_hints,
            ..Default::default()
        }
    }
}
