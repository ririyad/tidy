use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use rusqlite::Connection;
use serde::Serialize;
use thiserror::Error;

const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");
const DEFAULT_CONFIG: &str = r#"# Tidy vault configuration
schema_version = 1

[reader]
theme = "system"
font = "serif"
font_size = 18
line_height = 1.7
"#;
const DEFAULT_SOURCES: &str = "# Tidy sources\nschema_version = 1\nsources = []\n";

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("the selected path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("failed to access the vault filesystem: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to initialize the vault index: {0}")]
    Database(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultSummary {
    pub path: PathBuf,
    pub database_path: PathBuf,
    pub created: bool,
}

#[derive(Debug)]
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    pub fn initialize(path: impl AsRef<Path>) -> Result<VaultSummary, VaultError> {
        let root = path.as_ref();
        if root.exists() && !root.is_dir() {
            return Err(VaultError::NotDirectory(root.to_path_buf()));
        }

        let metadata_dir = root.join(".tidy");
        let created = !metadata_dir.exists();

        for directory in [
            root.to_path_buf(),
            metadata_dir.clone(),
            metadata_dir.join("cache"),
            metadata_dir.join("logs"),
            root.join("Sources"),
            root.join("attachments"),
        ] {
            fs::create_dir_all(directory)?;
        }

        write_if_missing(&metadata_dir.join("config.toml"), DEFAULT_CONFIG)?;
        write_if_missing(&metadata_dir.join("sources.toml"), DEFAULT_SOURCES)?;

        let database_path = metadata_dir.join("index.db");
        let connection = Connection::open(&database_path)?;
        connection.execute_batch(INITIAL_MIGRATION)?;

        Ok(VaultSummary {
            path: root.to_path_buf(),
            database_path,
            created,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, VaultError> {
        let root = path.as_ref();
        if !root.is_dir() {
            return Err(VaultError::NotDirectory(root.to_path_buf()));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database_path(&self) -> PathBuf {
        self.root.join(".tidy").join("index.db")
    }
}

fn write_if_missing(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => file.write_all(contents.as_bytes()).inspect_err(|_| {
            let _ = fs::remove_file(path);
        }),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_an_idempotent_vault() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Reading");

        let first = Vault::initialize(&root).unwrap();
        let second = Vault::initialize(&root).unwrap();

        assert!(first.created);
        assert!(!second.created);
        assert!(root.join(".tidy/index.db").is_file());
        assert!(root.join(".tidy/config.toml").is_file());
        assert!(root.join(".tidy/sources.toml").is_file());
        assert!(root.join(".tidy/cache").is_dir());
        assert!(root.join(".tidy/logs").is_dir());
        assert!(root.join("Sources").is_dir());
        assert!(root.join("attachments").is_dir());

        let connection = Connection::open(second.database_path).unwrap();
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);

        let table_count: u32 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'articles'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
    }

    #[test]
    fn rejects_a_file_as_vault_root() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let error = Vault::initialize(temp.path()).unwrap_err();
        assert!(matches!(error, VaultError::NotDirectory(_)));
    }
}
