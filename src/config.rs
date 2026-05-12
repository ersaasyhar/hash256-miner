use ethers::types::Address;
use std::env;

pub const DEFAULT_RPC_URL: &str = "https://ethereum.publicnode.com";
pub const DEFAULT_CONTRACT_ADDRESS: &str = "0xAC7b5d06fa1e77D08aea40d46cB7C5923A87A0cc";
pub const DEFAULT_BATCH_SIZE: u64 = 8_000_000;
pub const DEFAULT_REFRESH_EVERY_BATCHES: u64 = 20;

#[derive(Clone, Copy, Debug)]
pub enum MinerMode {
    Auto,
    Gpu,
    Cpu,
}

impl MinerMode {
    pub fn from_env() -> Self {
        match env::var("MODE").unwrap_or_else(|_| "auto".to_string()).to_lowercase().as_str() {
            "gpu" => MinerMode::Gpu,
            "cpu" => MinerMode::Cpu,
            _ => MinerMode::Auto,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub mode: MinerMode,
    pub rpc_url: String,
    pub contract: Address,
    pub batch_size: u64,
    pub threads: usize,
    pub refresh_every_batches: u64,
    pub submit: bool,
    pub start_nonce: u64,
    pub private_key: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let mode = MinerMode::from_env();
        let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
        let contract: Address = env::var("CONTRACT_ADDRESS")
            .unwrap_or_else(|_| DEFAULT_CONTRACT_ADDRESS.to_string())
            .parse()?;
        let batch_size = env::var("BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        let threads = env::var("THREADS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| v > 0)
            .unwrap_or_else(num_cpus::get);
        let refresh_every_batches = env::var("REFRESH_EVERY_BATCHES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_REFRESH_EVERY_BATCHES);
        let submit = env::var("SUBMIT").ok().as_deref() == Some("1");
        let start_nonce = env::var("START_NONCE")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let private_key = env::var("PRIVATE_KEY")
            .map_err(|_| "Set PRIVATE_KEY in your environment before running")?;

        Ok(Self {
            mode,
            rpc_url,
            contract,
            batch_size,
            threads,
            refresh_every_batches,
            submit,
            start_nonce,
            private_key,
        })
    }
}
