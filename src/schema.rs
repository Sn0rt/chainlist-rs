//! Serde bindings for the chainid.network / ethereum-lists chains schema.
//!
//! Use `load_chains()` to parse the downloaded `chains.json` from the path
//! provided by `CHAINS_JSON_PATH` (set by the build script).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Top-level chain record as defined by chainid.network.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainRecord {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub chain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub rpc: Vec<String>,
    #[serde(default)]
    pub features: Vec<Feature>,
    #[serde(default)]
    pub faucets: Vec<String>,
    pub native_currency: NativeCurrency,
    #[serde(rename = "infoURL")]
    pub info_url: String,
    pub short_name: String,
    pub chain_id: u64,
    pub network_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slip44: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ens: Option<Ens>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explorers: Vec<Explorer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Parent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub red_flags: Vec<String>,
}

impl ChainRecord {
    /// Access the nested native currency.
    pub fn native_currency(&self) -> &NativeCurrency {
        &self.native_currency
    }

    /// Access RPC endpoints.
    pub fn rpc_endpoints(&self) -> &[String] {
        &self.rpc
    }

    /// Access faucet URLs.
    pub fn faucets(&self) -> &[String] {
        &self.faucets
    }

    /// Access feature flags.
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    /// Access explorers.
    pub fn explorers(&self) -> &[Explorer] {
        &self.explorers
    }

    /// Access red flags.
    pub fn red_flags(&self) -> &[String] {
        &self.red_flags
    }

    /// Access parent network if present.
    pub fn parent(&self) -> Option<&Parent> {
        self.parent.as_ref()
    }

    /// Access ENS registry if present.
    pub fn ens(&self) -> Option<&Ens> {
        self.ens.as_ref()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Feature {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCurrency {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ens {
    pub registry: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Explorer {
    pub name: String,
    pub url: String,
    pub standard: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parent {
    #[serde(rename = "type")]
    pub type_field: String,
    pub chain: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bridges: Vec<Bridge>,
}

impl Parent {
    /// Access bridges connecting this chain to its parent.
    pub fn bridges(&self) -> &[Bridge] {
        &self.bridges
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bridge {
    pub url: String,
}

/// Errors when loading the full schema JSON.
#[derive(Debug, Error)]
pub enum SchemaLoadError {
    #[error("CHAINS_JSON_PATH not set; build script should export it")]
    MissingPath,
    #[error("failed to read {0}: {1}")]
    Io(String, #[source] std::io::Error),
    #[error("failed to parse {0}: {1}")]
    Json(String, #[source] serde_json::Error),
}

/// Load the full chain list from the downloaded chains.json.
pub fn load_chains() -> Result<Vec<ChainRecord>, SchemaLoadError> {
    let path = option_env!("CHAINS_JSON_PATH").ok_or(SchemaLoadError::MissingPath)?;
    let text =
        std::fs::read_to_string(path).map_err(|e| SchemaLoadError::Io(path.to_string(), e))?;
    serde_json::from_str(&text).map_err(|e| SchemaLoadError::Json(path.to_string(), e))
}
