mod chain;
mod config;
mod miner;
mod pow;

use chain::{call_balance_of, resolve_state, MiningState};
use config::{AppConfig, MinerMode};
use ethers::prelude::*;
use ethers::types::{H256, U256};
use miner::cpu::mine_batch_cpu;
#[cfg(feature = "gpu")]
use miner::gpu::GpuMiner;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let cfg = AppConfig::from_env()?;

    let provider = Provider::<Http>::try_from(cfg.rpc_url.as_str())?;
    let chain_id = provider.get_chainid().await?;

    let wallet: LocalWallet = cfg
        .private_key
        .parse::<LocalWallet>()?
        .with_chain_id(chain_id.as_u64());
    let miner = wallet.address();
    let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));

    let mut state = resolve_state(&provider, chain_id, cfg.contract, miner).await?;

    #[cfg(feature = "gpu")]
    let mut gpu_miner = {
        let global_work = cfg.batch_size as usize;
        let kernel_src = std::fs::read_to_string("kernel.cl")?;
        match cfg.mode {
            MinerMode::Cpu => None,
            MinerMode::Gpu | MinerMode::Auto => match GpuMiner::new(&kernel_src, global_work) {
                Ok(m) => Some(m),
                Err(e) => {
                    if matches!(cfg.mode, MinerMode::Gpu) {
                        return Err(e);
                    }
                    println!("GPU init failed, falling back to CPU: {e}");
                    None
                }
            },
        }
    };

    #[cfg(not(feature = "gpu"))]
    let gpu_miner: Option<()> = {
        if matches!(cfg.mode, MinerMode::Gpu) {
            return Err("MODE=gpu requested but binary was built without `gpu` feature".into());
        }
        None
    };

    println!("Contract: {:?}", cfg.contract);
    println!("Miner: {miner:?}");
    println!("Chain ID: {chain_id}");
    println!("Mode: {}", if gpu_miner.is_some() { "gpu" } else { "cpu" });
    println!("Threads (CPU fallback): {}", cfg.threads);
    let eth_wei = provider.get_balance(miner, None).await?;
    let eth_fmt = ethers::utils::format_units(eth_wei, 18)?;
    println!("Wallet ETH balance: {eth_fmt} ETH");
    match call_balance_of(&provider, cfg.contract, miner).await {
        Ok(v) => {
            let hash_fmt = ethers::utils::format_units(v, 18)?;
            println!("Wallet HASH balance: {hash_fmt} HASH");
        }
        Err(e) => println!("Wallet HASH balance: unavailable ({e})"),
    }
    print_state(&state);

    let mut current_nonce = cfg.start_nonce;
    let started = Instant::now();
    let mut searched_total: u128 = 0;
    let mut batches_since_refresh: u64 = 0;

    loop {
        #[cfg(feature = "gpu")]
        let solution = if let Some(gpu) = &mut gpu_miner {
            match gpu.mine_batch(state.challenge, state.target_be, current_nonce) {
                Ok(v) => v,
                Err(err) => {
                    if matches!(cfg.mode, MinerMode::Gpu) {
                        return Err(err);
                    }
                    println!("GPU mining failed, switching to CPU fallback: {err}");
                    gpu_miner = None;
                    mine_batch_cpu(
                        state.challenge,
                        state.target_be,
                        current_nonce,
                        cfg.batch_size,
                        cfg.threads,
                    )
                }
            }
        } else {
            mine_batch_cpu(
                state.challenge,
                state.target_be,
                current_nonce,
                cfg.batch_size,
                cfg.threads,
            )
        };

        #[cfg(not(feature = "gpu"))]
        let solution = mine_batch_cpu(
            state.challenge,
            state.target_be,
            current_nonce,
            cfg.batch_size,
            cfg.threads,
        );

        if let Some(solution) = solution {
            println!("Found nonce: {solution}");

            if cfg.submit {
                let tx_hash = submit_solution(&provider, &client, cfg.contract, miner, solution).await?;
                if let Some(tx_hash) = tx_hash {
                    println!("Submitted mine(uint64) tx: {tx_hash:?}");
                }
            }

            state = refresh_state(&provider, chain_id, cfg.contract, miner, &state).await?;
            current_nonce = solution.saturating_add(1);
            continue;
        }

        searched_total = searched_total.saturating_add(cfg.batch_size as u128);
        current_nonce = current_nonce.saturating_add(cfg.batch_size);
        batches_since_refresh = batches_since_refresh.saturating_add(1);

        let elapsed = started.elapsed().as_secs_f64();
        let hashes = searched_total as f64;
        let hps = if elapsed > 0.0 { hashes / elapsed } else { 0.0 };
        println!("searched={current_nonce} rate={:.0} H/s", hps);

        if batches_since_refresh >= cfg.refresh_every_batches {
            batches_since_refresh = 0;
            state = refresh_state(&provider, chain_id, cfg.contract, miner, &state).await?;
        }
    }
}

async fn submit_solution(
    provider: &Provider<Http>,
    client: &Arc<SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>>,
    contract: Address,
    miner: Address,
    solution: u64,
) -> Result<Option<H256>, Box<dyn std::error::Error>> {
    let selector = &ethers::utils::id("mine(uint64)")[0..4];
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(selector);
    let mut arg = [0u8; 32];
    arg[24..].copy_from_slice(&solution.to_be_bytes());
    data.extend_from_slice(&arg);

    let tx = TransactionRequest::new()
        .from(miner)
        .to(contract)
        .data(Bytes::from(data));

    match provider.call(&tx.clone().into(), None).await {
        Ok(_) => {
            let pending = client.send_transaction(tx, None).await?;
            Ok(Some(*pending))
        }
        Err(err) => {
            println!("Preflight eth_call reverted; not submitting tx: {err}. Refreshing state.");
            Ok(None)
        }
    }
}

async fn refresh_state(
    provider: &Provider<Http>,
    chain_id: U256,
    contract: Address,
    miner: Address,
    prev: &MiningState,
) -> Result<MiningState, Box<dyn std::error::Error>> {
    let next = resolve_state(provider, chain_id, contract, miner).await?;
    if prev.epoch != next.epoch || prev.difficulty_target != next.difficulty_target {
        println!(
            "State updated: epoch={} target=0x{:x}",
            next.epoch, next.difficulty_target
        );
    }
    Ok(next)
}

fn print_state(state: &MiningState) {
    println!("Epoch: {}", state.epoch);
    println!("Difficulty target: 0x{:x}", state.difficulty_target);
    println!("Challenge: 0x{}", hex::encode(state.challenge));
}
