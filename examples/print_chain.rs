//! Print a chain's native currency and explorer URLs using both the enum and the full schema.

use chainlist_rs::Chain;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Quick lookup via enum
    let chain = Chain::Mainnet;
    println!(
        "{} (id {}) uses {}",
        chain.name(),
        chain.id(),
        chain.native_currency().1
    );
    println!("Hex chain id (3085/3326): {}", chain.chain_id_hex());

    // EIP-3085: params for wallet_addEthereumChain
    let params = chain.to_eip3085();
    println!("EIP-3085 params (hex id): {}", params.chain_id);
    println!("  rpcUrls: {}", params.rpc_urls.len());
    println!(
        "  blockExplorerUrls: {}",
        params.block_explorer_urls.join(", ")
    );

    // Full schema access from bundled JSON (no filesystem path needed)
    let chains = chainlist_rs::all_chains();
    if let Some(mainnet) = chains.iter().find(|c| c.chain_id == chain.id()) {
        println!("RPC endpoints: {}", mainnet.rpc_endpoints().len());
        if let Some(explorer) = mainnet.explorers().first() {
            println!("First explorer: {} -> {}", explorer.name, explorer.url);
        }
        if let Some(parent) = mainnet.parent() {
            println!("Parent chain: {}", parent.chain);
            println!("Bridges: {}", parent.bridges().len());
        } else {
            println!("Parent chain: <none>");
        }
        println!("Full record:\n{}", serde_json::to_string_pretty(mainnet)?);
    }

    Ok(())
}
