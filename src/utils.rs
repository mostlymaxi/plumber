use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

pub enum PlumberFileError {
    NotFound,
    TomlError(toml::de::Error),
    IoError(std::io::Error),
    #[allow(dead_code)]
    MissingRequiredConfig(String),
}

impl From<std::io::Error> for PlumberFileError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => PlumberFileError::NotFound,
            _ => PlumberFileError::IoError(error),
        }
    }
}

impl From<toml::de::Error> for PlumberFileError {
    fn from(error: toml::de::Error) -> Self {
        PlumberFileError::TomlError(error)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlumberFileConfig {
    pub exec: String,
    pub run: Option<PlumberFileConfigRun>,
    pub metadata: Option<PlumberFileConfigMeta>,
    pub extra: Option<toml::Table>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlumberFileConfigRun {
    instances: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlumberFileConfigMeta {
    pub name: Option<String>,
    pub logging_dir: Option<String>,
    pub metadata_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlumberFile {
    pub path: PathBuf,
    pub name: String,
    pub config: PlumberFileConfig,
}

impl TryFrom<PathBuf> for PlumberFile {
    type Error = PlumberFileError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let data = fs::read_to_string(&path)?;
        let config = toml::from_str::<PlumberFileConfig>(&data)?;
        let name = match config.metadata.as_ref().and_then(|m| m.name.clone()) {
            Some(name) => name,
            None => {
                let name = path.file_stem().ok_or(PlumberFileError::NotFound)?;

                name.to_str().ok_or(PlumberFileError::NotFound)?.to_string()
            }
        };
        Ok(PlumberFile { path, name, config })
    }
}

impl PlumberFile {
    pub fn save_to(&self, path: &Path) {
        log::debug!("saving PlumberFile struct to {}", path.display());
        fs::create_dir_all(path).unwrap();

        let pf_raw = serde_json::to_vec(self)
            .expect("failed to serialize PlumberFile struct");

        let mut f = fs::File::create(path.join(".data"))
            .expect("failed to create file to store PlumberFile struct");

        f.write_all(&pf_raw).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn plumber_file_not_exist() {
        init();
        let nonexistent_path = PathBuf::from("./thisdoesntexist/definetly");
        assert!(PlumberFile::try_from(nonexistent_path).is_err());
    }

    #[test]
    fn test_toml() {
        init();
        let t: PlumberFileConfig = toml::from_str(r#"
        exec = "test | test | test"

        [metadata]
        name = "test"

        [run]
        instances = 3

        [extra]
        pizza = 'yes'

        "#).unwrap();

        log::info!("{:#?}", t);
    }
}
