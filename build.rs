use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

// Chain definitions from chainid.network
pub type Root = Vec<ChainInfo>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    pub name: String,
    pub chain: String,
    pub icon: Option<String>,
    pub rpc: Vec<String>,
    #[serde(default)]
    pub features: Vec<Feature>,
    pub faucets: Vec<String>,
    pub native_currency: NativeCurrency,
    #[serde(rename = "infoURL")]
    pub info_url: String,
    pub short_name: String,
    pub chain_id: i64,
    pub network_id: i64,
    pub slip44: Option<i64>,
    pub ens: Option<Ens>,
    #[serde(default)]
    pub explorers: Vec<Explorer>,
    pub title: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub red_flags: Vec<String>,
    pub parent: Option<Parent>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCurrency {
    pub name: String,
    pub symbol: String,
    pub decimals: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ens {
    pub registry: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Explorer {
    pub name: String,
    pub url: String,
    pub standard: String,
    pub icon: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parent {
    #[serde(rename = "type")]
    pub type_field: String,
    pub chain: String,
    #[serde(default)]
    pub bridges: Vec<Bridge>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bridge {
    pub url: String,
}

/// Simplified chain data for code generation
#[derive(Debug, Clone)]
struct ChainData {
    id: u64,
    name: String,
    name_str: String,
    short_name: String,
    rpc_urls: Vec<String>,
    features: Vec<String>,
    faucets: Vec<String>,
    info_url: String,
    icon: Option<String>,
    explorers: Vec<Explorer>,
    native_currency_name: String,
    native_currency_symbol: String,
    native_currency_decimals: u8,
    slip44: Option<i64>,
    block_time_ms: u64,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CHAINS_JSON_PATH");
    println!("cargo:rerun-if-env-changed=CHAINS_JSON_URL");

    let chains_json = load_chains_json();

    // Generate the code
    let chain_code = generate_chain_code(&chains_json);

    // Format the generated code
    let formatted_code = format_rust_code(&chain_code);

    // Write the generated code to a file
    let out_dir = env::var("OUT_DIR").unwrap();
    // Keep a copy of chains.json in OUT_DIR for include_str! and runtime helpers
    let json_dest = Path::new(&out_dir).join("chains.json");
    fs::write(&json_dest, &chains_json)
        .unwrap_or_else(|e| panic!("Failed to write chains.json to {:?}: {e}", json_dest));
    println!("cargo:rustc-env=CHAINS_JSON_PATH={}", json_dest.display());

    let dest_path = Path::new(&out_dir).join("chain_generated.rs");
    fs::write(&dest_path, formatted_code).unwrap();

    println!("cargo:info=Generated Chain enum from chainid.network/chains.json");
}

fn load_chains_json() -> String {
    // Prefer env override for reproducibility in CI or vendored builds
    if let Ok(path) = env::var("CHAINS_JSON_PATH") {
        println!("cargo:rustc-env=CHAINS_JSON_PATH={path}");
        return fs::read_to_string(path).expect("Failed to read CHAINS_JSON_PATH file");
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by Cargo"));
    let cache_dir = chains_json_dir(&manifest_dir);
    let local = cache_dir.join("chains.json");
    println!("cargo:rerun-if-changed={}", local.display());

    // In docs.rs or offline builds, use local file without TTL check
    let is_docs_rs = env::var("DOCS_RS").is_ok();
    let is_offline = env::var("CARGO_FEATURE_OFFLINE").is_ok();

    if is_docs_rs || is_offline {
        if local.exists() {
            return fs::read_to_string(&local)
                .expect("Failed to read local chains.json in offline mode");
        } else {
            panic!(
                "chains.json not found at {:?} and network access is disabled",
                local
            );
        }
    }

    let ttl = Duration::from_secs(2 * 60 * 60); // 2h

    if !is_stale(&local, ttl) {
        if let Ok(text) = fs::read_to_string(&local) {
            return text;
        }
    }

    let url = env::var("CHAINS_JSON_URL")
        .unwrap_or_else(|_| "https://chainid.network/chains.json".to_string());

    // Try to download, fallback to local file if download fails
    match download_chains_json(&url) {
        Some(text) => {
            if let Some(parent) = local.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    panic!("Failed to create chains.json directory {:?}: {e}", parent);
                }
            }
            fs::write(&local, &text).unwrap_or_else(|e| {
                panic!("Failed to write downloaded chains.json to {:?}: {e}", local)
            });
            text
        }
        None => {
            // Fallback to local file
            if local.exists() {
                println!(
                    "cargo:warning=Network download failed, using local chains.json at {:?}",
                    local
                );
                fs::read_to_string(&local)
                    .expect("Failed to read local chains.json after network failure")
            } else {
                panic!(
                    "Failed to download chains.json from {} and no local file exists at {:?}",
                    url, local
                );
            }
        }
    }
}

fn download_chains_json(url: &str) -> Option<String> {
    let client = match Client::builder().timeout(Duration::from_secs(30)).build() {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Failed to build HTTP client: {e}");
            return None;
        }
    };

    let response = match client
        .get(url)
        .header("User-Agent", "chainlist-rs/0.1")
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            println!("cargo:warning=Failed to download {url}: {e}");
            return None;
        }
    };

    if !response.status().is_success() {
        println!(
            "cargo:warning=Fetching {url} returned HTTP {}",
            response.status()
        );
        return None;
    }

    match response.text() {
        Ok(text) => Some(text),
        Err(e) => {
            println!("cargo:warning=Failed to read response body from {url}: {e}");
            None
        }
    }
}

fn is_stale(path: &Path, ttl: Duration) -> bool {
    match fs::metadata(path) {
        Ok(meta) => match meta.modified() {
            Ok(modified) => match modified.elapsed() {
                Ok(elapsed) => elapsed > ttl,
                Err(_) => true,
            },
            Err(_) => true,
        },
        Err(_) => true,
    }
}

fn chains_json_dir(manifest_dir: &Path) -> PathBuf {
    if let Ok(dir) = env::var("CHAINS_JSON_DIR") {
        return PathBuf::from(dir);
    }
    let nested = manifest_dir.join("data");
    if nested.is_dir() {
        nested
    } else {
        manifest_dir.to_path_buf()
    }
}

fn generate_chain_code(json_str: &str) -> String {
    let chains: Root = match serde_json::from_str(json_str) {
        Ok(chains) => chains,
        Err(err) => panic!("Failed to parse chains.json: {err}"),
    };

    // Process chain data
    let mut chain_data = get_chains(&chains);
    chain_data.sort_by_key(|c| c.id);

    // Generate enum variants
    let mut enum_variants = TokenStream::new();

    // Regular chain variants
    for chain in &chain_data {
        let name_ident = format_ident!("{}", chain.name);
        let doc_comment = format!("{} (Chain ID: {})", chain.name_str, chain.id);
        let variant = quote! {
            #[doc = #doc_comment]
            #name_ident,
        };
        enum_variants.extend(variant);
    }

    // Generate chain info entries for the match statement
    let chain_info_entries = chain_data
        .iter()
        .map(|chain| {
            let name_ident = format_ident!("{}", chain.name);
            let id = chain.id;
            let name_str = &chain.name_str;
            let short_name = &chain.short_name;
            let info_url = &chain.info_url;
            let features = if chain.features.is_empty() {
                quote! { vec![] }
            } else {
                let feature_items = chain.features.iter().collect::<Vec<_>>();
                quote! { vec![#(#feature_items.to_string()),*] }
            };
            let currency_name = &chain.native_currency_name;
            let currency_symbol = &chain.native_currency_symbol;
            let decimals = chain.native_currency_decimals;
            let block_time = chain.block_time_ms;
            let icon = if let Some(icon) = &chain.icon {
                quote! { Some(#icon.to_string()) }
            } else {
                quote! { None }
            };

            let explorers = if chain.explorers.is_empty() {
                quote! { vec![] }
            } else {
                let explorer_items = chain
                    .explorers
                    .iter()
                    .map(|e| {
                        let name = e.name.clone();
                        let url = e.url.clone();
                        let standard = e.standard.clone();
                        let icon = if let Some(icon) = &e.icon {
                            quote! { Some(#icon.to_string()) }
                        } else {
                            quote! { None }
                        };
                        quote! {
                            Explorer {
                                name: #name.to_string(),
                                url: #url.to_string(),
                                standard: #standard.to_string(),
                                icon: #icon,
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                quote! { vec![#(#explorer_items),*] }
            };

            // Generate RPC URLs
            let rpc_urls = if chain.rpc_urls.is_empty() {
                quote! { vec![] }
            } else {
                let urls: Vec<_> = chain.rpc_urls.iter().collect();
                quote! { vec![#(#urls),*] }
            };

            // Generate faucets
            let faucets = if chain.faucets.is_empty() {
                quote! { vec![] }
            } else {
                let faucet_items = chain
                    .faucets
                    .iter()
                    .map(|url| {
                        let url_str = url.clone();
                        quote! { #url_str.to_string() }
                    })
                    .collect::<Vec<_>>();
                quote! { vec![#(#faucet_items),*] }
            };

            // Generate slip44
            let slip44 = if let Some(slip) = chain.slip44 {
                quote! { Some(#slip) }
            } else {
                quote! { None }
            };

            quote! {
                Self::#name_ident => ChainInfo {
                    id: #id,
                    name: #name_str,
                    short_name: #short_name,
                    rpc_urls: #rpc_urls,
                    features: #features,
                    faucets: #faucets,
                    native_currency: NativeCurrency {
                        name: #currency_name.to_string(),
                        symbol: #currency_symbol.to_string(),
                        decimals: #decimals,
                    },
                    info_url: #info_url,
                    slip44: #slip44,
                    block_time_ms: #block_time,
                    icon: #icon,
                    explorers: #explorers,
                }
            }
        })
        .collect::<Vec<_>>();

    // Generate TryFrom match arms for the try_from implementation
    let try_from_arms = chain_data
        .iter()
        .map(|chain| {
            let name_ident = format_ident!("{}", chain.name);
            let id = chain.id;

            quote! {
                #id => Ok(Self::#name_ident),
            }
        })
        .collect::<Vec<_>>();

    // Combine all the parts using quote!
    let generated_code = quote! {
        use crate::schema::{ChainRecord, Explorer, NativeCurrency};
        use once_cell::sync::OnceCell;
        use strum_macros::EnumIter;

        #[doc = r" Chain metadata derived from chainid.network"]
        #[derive(Debug, Clone)]
        pub struct ChainInfo {
            pub id: u64,
            pub name: &'static str,
            pub short_name: &'static str,
            pub rpc_urls: Vec<&'static str>,
            pub features: Vec<String>,
            pub faucets: Vec<String>,
            pub native_currency: NativeCurrency,
            pub info_url: &'static str,
            pub slip44: Option<i64>,
            pub block_time_ms: u64,
            pub icon: Option<String>,
            pub explorers: Vec<Explorer>,
        }

        #[doc = r" The Chain enum represents various blockchain networks."]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
        pub enum Chain {
            #enum_variants
        }

        static CHAINS_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/chains.json"));
        static CHAINS: OnceCell<Vec<ChainRecord>> = OnceCell::new();

        /// Full chain list deserialized from the bundled chains.json.
        pub fn all_chains() -> &'static [ChainRecord] {
            CHAINS
                .get_or_init(|| {
                    serde_json::from_str(CHAINS_JSON)
                        .expect("Failed to parse bundled chains.json; try cleaning and rebuilding")
                })
                .as_slice()
        }

        impl Chain {
            /// Returns chain information
            pub fn info(&self) -> ChainInfo {
                match self {
                    #(#chain_info_entries),*
                }
            }

            /// Returns the numerical ID of this chain.
            pub fn id(&self) -> u64 {
                self.info().id
            }

            /// Returns the canonical name of this chain.
            pub fn name(&self) -> &'static str {
                self.info().name
            }

            /// Returns a list of RPC URLs for this chain.
            pub fn rpc_urls(&self) -> Vec<&'static str> {
                self.info().rpc_urls
            }

            /// Returns the list of features supported by the chain
            pub fn features(&self) -> Vec<String> {
                self.info().features
            }

            /// Returns the list of faucet URLs for the chain
            pub fn faucets(&self) -> Vec<String> {
                self.info().faucets
            }

            /// Returns the native currency details (name, symbol, decimals) as a tuple
            pub fn native_currency(&self) -> (String, String, u8) {
                let currency = self.native_currency_info();
                (currency.name, currency.symbol, currency.decimals)
            }

            /// Returns the native currency as a structured value
            pub fn native_currency_info(&self) -> NativeCurrency {
                self.info().native_currency
            }

            /// Returns the information URL for the chain
            pub fn info_url(&self) -> &'static str {
                self.info().info_url
            }

            /// Returns the short name of the chain
            pub fn short_name(&self) -> &'static str {
                self.info().short_name
            }

            /// Returns the SLIP-44 coin type for the chain, if available
            pub fn slip44(&self) -> Option<i64> {
                self.info().slip44
            }

            /// Returns the block time in milliseconds
            pub fn block_time_in_ms(&self) -> Duration {
                Duration::from_millis(self.info().block_time_ms)
            }

            /// Returns the number of blocks that fits into the given time (in milliseconds)
            pub fn blocks_in(&self, time_in_ms: u64) -> f64 {
                time_in_ms as f64 / self.block_time_in_ms().as_millis() as f64
            }
        }

        impl TryFrom<u64> for Chain {
            type Error = ChainIdNotSupported;

            /// Initializes `Chain` from a chain ID, returns error if the chain id is not supported
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms)*
                    // Other chain IDs not supported
                    _ => Err(ChainIdNotSupported),
                }
            }
        }

        impl TryFrom<U256> for Chain {
            type Error = ChainIdNotSupported;

            /// Initializes `Chain` from a chain ID, returns error if the chain id is not supported
            fn try_from(value: U256) -> Result<Self, Self::Error> {
                // Check to avoid panics for large `U256` values
                if value > U256::from(u64::MAX) {
                    return Err(ChainIdNotSupported);
                }
                // Convert U256 to u64 using TryFrom trait rather than as_u64 method
                match u64::try_from(value) {
                    Ok(id) => Self::try_from(id),
                    Err(_) => Err(ChainIdNotSupported),
                }
            }
        }

        impl<'de> Deserialize<'de> for Chain {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct NetworkVisitor;

                impl de::Visitor<'_> for NetworkVisitor {
                    type Value = Chain;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("a u64 or a string")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Chain::try_from(value).map_err(E::custom)
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Chain::try_from(value.parse::<u64>().map_err(E::custom)?).map_err(E::custom)
                    }
                }

                deserializer.deserialize_any(NetworkVisitor)
            }
        }

        #[doc = r" Error indicating that a particular chain ID is not supported."]
        #[derive(Error, Debug)]
        #[error("chain id not supported")]
        pub struct ChainIdNotSupported;
    };

    generated_code.to_string()
}

// Returns a list of chains with their data from the chains.json file
fn get_chains(chains: &[ChainInfo]) -> Vec<ChainData> {
    // Default chain names (used for known chains to ensure consistent naming)
    let default_names = HashMap::from([
        (1, "Mainnet"),
        (56, "Bnb"),
        (100, "Gnosis"),
        (11155111, "Sepolia"),
        (8453, "Base"),
        (31337, "Hardhat"),
    ]);

    // Block times for specific chains
    let block_times = HashMap::from([
        (1, 12_000),        // Ethereum Mainnet: 12 seconds
        (11155111, 12_000), // Sepolia: 12 seconds
        (100, 5_000),       // Gnosis: 5 seconds
        (8453, 2_000),      // Base: 2 seconds
    ]);

    // Process all chains from the JSON file
    chains
        .iter()
        .map(|chain| {
            // Get default enum variant name if it's a known chain, otherwise generate one
            let name = default_names
                .get(&chain.chain_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| sanitize_enum_name(&chain.short_name, chain.chain_id));

            // Get specific block time for known chains or use default 12 seconds
            let block_time_ms = *block_times.get(&chain.chain_id).unwrap_or(&12_000);

            ChainData {
                id: chain.chain_id as u64,
                name,
                name_str: chain.name.clone(),
                short_name: chain.short_name.clone(),
                rpc_urls: chain.rpc.clone(),
                features: chain.features.iter().map(|f| f.name.clone()).collect(),
                faucets: chain.faucets.clone(),
                info_url: chain.info_url.clone(),
                icon: chain.icon.clone(),
                explorers: chain.explorers.clone(),
                native_currency_name: chain.native_currency.name.clone(),
                native_currency_symbol: chain.native_currency.symbol.clone(),
                native_currency_decimals: chain.native_currency.decimals as u8,
                slip44: chain.slip44,
                block_time_ms,
            }
        })
        .collect::<Vec<ChainData>>()
}

fn sanitize_enum_name(name: &str, chain_id: i64) -> String {
    let mut filtered: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    filtered = filtered
        .split('_')
        .map(|word| {
            if !word.is_empty() {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            } else {
                String::new()
            }
        })
        .collect();

    if filtered.is_empty() || !filtered.chars().next().unwrap().is_alphabetic() {
        format!("Chain{chain_id}")
    } else {
        filtered
    }
}

/// Formats the Rust code string using syn and prettyplease.
/// This is much better than the manual formatting approach because it properly
/// understands Rust syntax structures.
fn format_rust_code(code: &str) -> String {
    // Try to parse the code with syn
    match syn::parse_file(code) {
        Ok(file) => {
            // If parsing succeeds, use prettyplease to format the code
            prettyplease::unparse(&file)
        }
        Err(_) => code.to_string(),
    }
}
