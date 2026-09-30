use hnsw_filtered_vector_index::index::hnsw::HnswIndex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing HNSW Vector Index Engine (dim=128, M=16, efConstruction=64)...");
    let index = HnswIndex::new(128, 16, 64);
    tracing::info!("Index initialized successfully. Ready for vectors.");
    let _ = index;
    Ok(())
}
