//! EIP-compatible structures and conversions.

use crate::schema::{ChainRecord, NativeCurrency};
use crate::Chain;
use serde::{Deserialize, Serialize};

/// EIP-3085 wallet addChain parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip3085Params {
    /// Hex string chain ID, e.g. "0x1".
    pub chain_id: String,
    pub chain_name: String,
    pub native_currency: NativeCurrency,
    pub rpc_urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block_explorer_urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_urls: Vec<String>,
}

impl Eip3085Params {
    fn from_parts(
        chain_id: u64,
        chain_name: &str,
        native_currency: &NativeCurrency,
        rpc_urls: &[impl AsRef<str>],
        explorer_urls: &[String],
        icon_urls: &[String],
    ) -> Self {
        Self {
            chain_id: format!("0x{:x}", chain_id),
            chain_name: chain_name.to_string(),
            native_currency: native_currency.clone(),
            rpc_urls: rpc_urls.iter().map(|s| s.as_ref().to_string()).collect(),
            block_explorer_urls: explorer_urls.to_vec(),
            icon_urls: icon_urls.to_vec(),
        }
    }
}

impl Chain {
    /// Hex chain ID string (usable for EIP-3085/3326).
    pub fn chain_id_hex(&self) -> String {
        format!("0x{:x}", self.id())
    }

    /// Convert to EIP-3085 wallet parameters.
    pub fn to_eip3085(&self) -> Eip3085Params {
        let info = self.info();
        let explorer_urls: Vec<String> = info
            .explorers
            .iter()
            .filter(|e| e.standard == "EIP3091" || e.standard.is_empty())
            .map(|e| e.url.clone())
            .collect();
        let icon_urls = info.icon.iter().cloned().collect::<Vec<_>>();
        Eip3085Params::from_parts(
            info.id,
            info.name,
            &info.native_currency,
            &info.rpc_urls,
            &explorer_urls,
            &icon_urls,
        )
    }
}

impl ChainRecord {
    /// Hex chain ID string (usable for EIP-3085/3326).
    pub fn chain_id_hex(&self) -> String {
        format!("0x{:x}", self.chain_id)
    }

    /// Convert schema record to EIP-3085 wallet parameters.
    pub fn to_eip3085(&self) -> Eip3085Params {
        let explorer_urls: Vec<String> = self
            .explorers
            .iter()
            .filter(|e| e.standard == "EIP3091" || e.standard.is_empty())
            .map(|e| e.url.clone())
            .collect();
        let icon_urls = self.icon.iter().cloned().collect::<Vec<_>>();
        Eip3085Params::from_parts(
            self.chain_id,
            &self.name,
            &self.native_currency,
            &self.rpc,
            &explorer_urls,
            &icon_urls,
        )
    }
}
