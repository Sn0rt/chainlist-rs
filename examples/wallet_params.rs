//! Show EIP-3085 wallet_addEthereumChain parameters for a chain.

use chainlist_rs::Chain;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chain = Chain::Mainnet;
    let params = chain.to_eip3085();

    println!("chainId: {}", params.chain_id);
    println!("chainName: {}", params.chain_name);
    println!(
        "nativeCurrency: {} ({}) decimals {}",
        params.native_currency.name, params.native_currency.symbol, params.native_currency.decimals
    );
    println!("rpcUrls: {}", params.rpc_urls.join(", "));
    println!(
        "blockExplorerUrls: {}",
        params.block_explorer_urls.join(", ")
    );

    Ok(())
}
