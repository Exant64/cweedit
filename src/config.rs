use std::path::PathBuf;

use serde_json::{json, Value};

const CONFIG_FILE_PATH: &'static str = "config.json";

pub struct Config {
    auto_save: bool,
    auto_load_found_texture: bool,

    last_object_folder: Option<PathBuf>,
    last_texture_folder: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_save: true,
            auto_load_found_texture: false,
            last_object_folder: None,
            last_texture_folder: None,
        }
    }
}

impl Config {
    // returns default if file doesn't exist
    // returns error if failed to parse
    pub fn init() -> Result<Self, String> {
        let document_text = std::fs::read_to_string(CONFIG_FILE_PATH);

        if let Ok(file_contents) = document_text {
            let document: Value = serde_json::from_str(file_contents.as_str())
                .map_err(|err| format!("Error parsing config JSON: {}", err))?;
            let mut config = Self::default();

            config.auto_save = if let Some(auto_save) = document["auto_save"].as_bool() {
                Ok(auto_save)
            } else {
                Err("configuration is not a bool!".to_string())
            }?;

            config.auto_load_found_texture = if let Some(auto_load_found_texture) =
                document["auto_load_found_texture"].as_bool()
            {
                Ok(auto_load_found_texture)
            } else {
                Err("auto_load_found_texture is not a bool!".to_string())
            }?;

            config.last_object_folder = document["last_object_folder"]
                .as_str()
                .map(|p| PathBuf::from(p));
            config.last_texture_folder = document["last_texture_folder"]
                .as_str()
                .map(|p| PathBuf::from(p));

            return Ok(config);
        }

        Ok(Self::default())
    }

    pub fn save_config(&mut self) {
        let contents = json!({
            "auto_save": self.auto_save,
            "auto_load_found_texture": self.auto_load_found_texture,
            "last_object_folder": self.last_object_folder,
            "last_texture_folder": self.last_texture_folder
        });

        if let Err(err) = std::fs::write(CONFIG_FILE_PATH, contents.to_string()) {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Error")
                .set_description(format!("Failed to save editor config: {}", err))
                .show();
        }
    }
}
