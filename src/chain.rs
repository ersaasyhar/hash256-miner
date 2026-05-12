use crate::pow::keccak256;
use ethers::abi::{encode, Token};
use ethers::prelude::*;
use ethers::types::{Address, U256};
use std::env;

const EPOCH_BLOCK_SPAN: u64 = 100;

pub fn challenge_for(chain_id: U256, contract: Address, miner: Address, epoch: U256) -> [u8; 32] {
    let packed = encode(&[
        Token::Uint(chain_id),
        Token::Address(contract),
        Token::Address(miner),
        Token::Uint(epoch),
    ]);
    keccak256(&packed)
}

pub fn parse_env_u256(name: &str) -> Result<U256, Box<dyn std::error::Error>> {
    let raw = env::var(name)?;
    if let Some(hex) = raw.strip_prefix("0x") {
        let bytes = hex::decode(hex)?;
        if bytes.len() > 32 {
            return Err(format!("{name} hex value is larger than 32 bytes").into());
        }
        let mut be = [0u8; 32];
        be[32 - bytes.len()..].copy_from_slice(&bytes);
        Ok(U256::from_big_endian(&be))
    } else {
        Ok(U256::from_dec_str(&raw)?)
    }
}

pub async fn call_u256(
    provider: &Provider<Http>,
    to: Address,
    sig: &str,
) -> Result<U256, Box<dyn std::error::Error>> {
    let selector = &ethers::utils::id(sig)[0..4];
    let tx = TransactionRequest::new().to(to).data(Bytes::from(selector.to_vec()));
    let raw = provider.call(&tx.into(), None).await?;
    if raw.0.len() < 32 {
        return Err(format!("short return data for {sig}").into());
    }
    Ok(U256::from_big_endian(&raw.0[..32]))
}

pub async fn call_balance_of(
    provider: &Provider<Http>,
    contract: Address,
    owner: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let selector = &ethers::utils::id("balanceOf(address)")[0..4];
    let encoded = encode(&[Token::Address(owner)]);
    let mut data = Vec::with_capacity(4 + encoded.len());
    data.extend_from_slice(selector);
    data.extend_from_slice(&encoded);

    let tx = TransactionRequest::new().to(contract).data(Bytes::from(data));
    let raw = provider.call(&tx.into(), None).await?;
    if raw.0.len() < 32 {
        return Err("short return data for balanceOf(address)".into());
    }
    Ok(U256::from_big_endian(&raw.0[..32]))
}

pub async fn detect_difficulty_target(
    provider: &Provider<Http>,
    contract: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let candidates = [
        "currentDifficulty()",
        "difficulty()",
        "miningTarget()",
        "currentTarget()",
    ];
    for sig in candidates {
        if let Ok(v) = call_u256(provider, contract, sig).await {
            return Ok(v);
        }
    }
    Err("failed to detect difficulty target automatically; set DIFFICULTY_TARGET".into())
}

pub async fn detect_epoch(
    provider: &Provider<Http>,
    contract: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let candidates = ["currentEpoch()", "epoch()", "miningEpoch()"];
    for sig in candidates {
        if let Ok(v) = call_u256(provider, contract, sig).await {
            return Ok(v);
        }
    }
    let block = provider.get_block_number().await?;
    Ok(U256::from(block.as_u64() / EPOCH_BLOCK_SPAN))
}

#[derive(Clone, Debug)]
pub struct MiningState {
    pub epoch: U256,
    pub difficulty_target: U256,
    pub challenge: [u8; 32],
    pub target_be: [u8; 32],
}

pub async fn resolve_state(
    provider: &Provider<Http>,
    chain_id: U256,
    contract: Address,
    miner: Address,
) -> Result<MiningState, Box<dyn std::error::Error>> {
    let epoch = match parse_env_u256("EPOCH") {
        Ok(v) => v,
        Err(_) => detect_epoch(provider, contract).await?,
    };
    let difficulty_target = match parse_env_u256("DIFFICULTY_TARGET") {
        Ok(v) => v,
        Err(_) => detect_difficulty_target(provider, contract).await?,
    };
    let challenge = challenge_for(chain_id, contract, miner, epoch);
    let mut target_be = [0u8; 32];
    difficulty_target.to_big_endian(&mut target_be);

    Ok(MiningState {
        epoch,
        difficulty_target,
        challenge,
        target_be,
    })
}
