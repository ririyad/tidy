use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub url: String,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct HttpCache {
    root: PathBuf,
}

impl HttpCache {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn load(&self, url: &Url) -> Result<Option<CacheEntry>> {
        let path = self.meta_path(url);
        if !path.exists() {
            return Ok(None);
        }
        let meta: CacheMeta = serde_json::from_slice(&fs::read(&path)?)
            .map_err(|error| crate::error::TidyError::Message(error.to_string()))?;
        let body = fs::read(self.body_path(url))?;
        Ok(Some(CacheEntry {
            url: meta.url,
            content_type: meta.content_type,
            etag: meta.etag,
            last_modified: meta.last_modified,
            body,
        }))
    }

    pub fn store(
        &self,
        url: &Url,
        body: &[u8],
        content_type: Option<&str>,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::write(self.body_path(url), body)?;
        let meta = CacheMeta {
            url: url.to_string(),
            content_type: content_type.map(str::to_owned),
            etag: etag.map(str::to_owned),
            last_modified: last_modified.map(str::to_owned),
        };
        let json = serde_json::to_vec_pretty(&meta)
            .map_err(|error| crate::error::TidyError::Message(error.to_string()))?;
        fs::write(self.meta_path(url), json)?;
        Ok(())
    }

    fn key(url: &Url) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_str().as_bytes());
        hex::encode(hasher.finalize())
    }

    fn meta_path(&self, url: &Url) -> PathBuf {
        self.root.join(format!("{}.json", Self::key(url)))
    }

    fn body_path(&self, url: &Url) -> PathBuf {
        self.root.join(format!("{}.bin", Self::key(url)))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheMeta {
    url: String,
    content_type: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
