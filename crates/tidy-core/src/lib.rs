mod vault;

pub use vault::{Vault, VaultError, VaultSummary};

pub const APP_NAME: &str = "Tidy";
pub const VAULT_SCHEMA_VERSION: u32 = 1;
