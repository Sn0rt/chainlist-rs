//! Chain metadata and helpers
//!
//! Access chain IDs, names, native currency, RPC URLs and convenience helpers.
//!
//! ## Build-time data
//!
//! By default the build script downloads `chains.json` from
//! `https://chainid.network/chains.json`. This requires network access during
//! compilation. Override with:
//! - `CHAINS_JSON_URL` to point to another source.
//! - `CHAINS_JSON_PATH` to supply a local file and skip the download.
//!
//! ## Examples
//!
//! ```rust
//! use chainlist_rs::Chain;
//!
//! let mainnet = Chain::Mainnet;
//! assert_eq!(mainnet.id(), 1);
//! println!("{} -> native {}", mainnet.name(), mainnet.native_currency().1);
//! ```

use alloy_primitives::U256;
use serde::{de, Deserialize, Deserializer};
use std::time::Duration;
use thiserror::Error;

pub mod eip;
pub mod schema;

include!(concat!(env!("OUT_DIR"), "/chain_generated.rs"));

#[cfg(test)]
mod test {
    use super::{all_chains, Chain};
    use crate::schema;
    use serde_json::Value;
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    #[test]
    fn test_chain_count_vs_json() {
        use std::fs;
        use std::path::PathBuf;

        let enum_count = Chain::iter().count();
        println!("enum_count: {enum_count}");

        let local_chain_ids: HashSet<u64> = Chain::iter().map(|chain| chain.id()).collect();

        // Load local chains.json shipped with the crate to keep tests offline and deterministic
        let path = option_env!("CHAINS_JSON_PATH")
            .map(PathBuf::from)
            .expect("CHAINS_JSON_PATH not set; build script should have downloaded chains.json");
        let json_text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));
        let chains: Vec<Value> = serde_json::from_str(&json_text).expect("Failed to parse JSON");
        let json_chain_count = chains.len();

        println!("chains.json contains {json_chain_count} chains");
        assert_eq!(
            enum_count, json_chain_count,
            "enum_count and json_chain_count should be equal"
        );

        if enum_count != json_chain_count {
            for chain_data in chains {
                if let Some(chain_id) = chain_data.get("chainId").and_then(|id| id.as_u64()) {
                    if !local_chain_ids.contains(&chain_id) {
                        let name = chain_data
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("Unknown");
                        let short_name = chain_data
                            .get("shortName")
                            .and_then(|n| n.as_str())
                            .unwrap_or("Unknown");
                        println!(
                            "Missing chain: ID={chain_id}, Name={name}, ShortName={short_name}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_chain_properties() {
        // Test for Mainnet
        let mainnet = Chain::Mainnet;
        assert_eq!(mainnet.id(), 1);
        assert!(mainnet.name().contains("Ethereum"));

        // Check native currency for a few chains
        let (_name, symbol, decimals) = mainnet.native_currency();
        assert_eq!(symbol.to_uppercase(), "ETH");
        assert_eq!(decimals, 18);

        // Test RPC URLs - should return non-empty list for mainnet
        assert!(!Chain::Mainnet.rpc_urls().is_empty());

        // Test info_url - should be valid URL
        assert!(Chain::Mainnet.info_url().starts_with("http"));

        // Test short name
        assert_eq!(Chain::Mainnet.short_name().to_uppercase(), "ETH");

        // Test SLIP44 value for Ethereum
        assert_eq!(Chain::Mainnet.slip44(), Some(60));
    }

    #[test]
    fn test_blocks_in() {
        const TARGET_AGE: u64 = 6 * 60 * 60 * 1000; // 6h in ms

        assert_eq!(Chain::Mainnet.blocks_in(TARGET_AGE).round(), 1800.0);
        assert_eq!(Chain::Sepolia.blocks_in(TARGET_AGE).round(), 1800.0);
        // Only check chains present in local chains.json
    }

    #[test]
    fn test_deserialize_from_str() {
        // Test valid string deserialization
        let json_data = "\"1\""; // Should parse to u64 1 and then to Network::Mainnet
        let network: Chain = serde_json::from_str(json_data).unwrap();
        assert_eq!(network, Chain::Mainnet);

        let json_data = "\"11155111\""; // Should parse to u64 11155111 and then to Network::Sepolia
        let network: Chain = serde_json::from_str(json_data).unwrap();
        assert_eq!(network, Chain::Sepolia);

        // Skip Gnosis: not present in current local chains.json

        // Test invalid string deserialization (should return an error)
        let json_data = "\"invalid\""; // Cannot be parsed as u64
        let result: Result<Chain, _> = serde_json::from_str(json_data);
        assert!(result.is_err());
    }

    #[test]
    fn chains_sorted_and_unique() {
        let ids: Vec<u64> = Chain::iter().map(|c| c.id()).collect();
        assert!(!ids.is_empty(), "Chain list should not be empty");

        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(
            ids, sorted,
            "Chain variants should stay ordered by chain id"
        );

        let unique: HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "Chain ids should be unique across the enum"
        );
    }

    #[test]
    fn chain_records_have_basic_fields() {
        let mut short_names = HashSet::new();

        for record in all_chains() {
            assert!(
                Chain::try_from(record.chain_id).is_ok(),
                "Chain::try_from should cover chain_id {}",
                record.chain_id
            );
            assert!(
                record.chain_id > 0,
                "chain_id should be positive for {}",
                record.name
            );
            assert!(
                !record.name.trim().is_empty(),
                "name should not be empty for chain_id {}",
                record.chain_id
            );
            assert!(
                !record.chain.trim().is_empty(),
                "chain slug should not be empty for chain_id {}",
                record.chain_id
            );
            assert!(
                !record.short_name.trim().is_empty(),
                "short_name should not be empty for chain_id {}",
                record.chain_id
            );
            assert!(
                short_names.insert(record.short_name.as_str()),
                "short_name {} reused for chain_id {}",
                record.short_name,
                record.chain_id
            );
            assert!(
                !record.native_currency.name.trim().is_empty(),
                "native currency name missing for chain_id {}",
                record.chain_id
            );
            assert!(
                !record.native_currency.symbol.trim().is_empty(),
                "native currency symbol missing for chain_id {}",
                record.chain_id
            );
            assert!(
                record.native_currency.decimals > 0,
                "native currency decimals must be >0 for chain_id {}",
                record.chain_id
            );
        }
    }

    #[test]
    fn schema_loader_matches_bundled_data() {
        let loaded = schema::load_chains().expect("schema::load_chains should succeed");
        let bundled = all_chains();

        assert_eq!(
            loaded.len(),
            bundled.len(),
            "schema loader should match bundled chain count"
        );

        let loaded_ids: HashSet<u64> = loaded.iter().map(|c| c.chain_id).collect();
        let bundled_ids: HashSet<u64> = bundled.iter().map(|c| c.chain_id).collect();
        assert_eq!(
            loaded_ids, bundled_ids,
            "schema loader and bundled data should agree on chain ids"
        );
    }
}
