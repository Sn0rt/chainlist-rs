//! Print a brief summary of the first few chains.

use chainlist_rs::schema::ChainRecord;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chains: &[ChainRecord] = chainlist_rs::all_chains();
    println!("Total chains: {}", chains.len());

    for chain in chains.iter().take(5) {
        let feature_names = chain
            .features()
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{} (id {}) features: [{}] explorers: {}",
            chain.name,
            chain.chain_id,
            feature_names,
            chain.explorers().len(),
        );
    }

    Ok(())
}
