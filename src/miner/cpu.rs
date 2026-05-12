use crate::pow::hash_meets_target_preimage;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub fn mine_batch_cpu(
    challenge: [u8; 32],
    target_be: [u8; 32],
    start_nonce: u64,
    batch_size: u64,
    threads: usize,
) -> Option<u64> {
    if batch_size == 0 || threads == 0 {
        return None;
    }

    let found = Arc::new(AtomicBool::new(false));
    let winning_nonce = Arc::new(AtomicU64::new(0));
    let challenge_arc = Arc::new(challenge);
    let target_arc = Arc::new(target_be);
    let threads_u64 = threads as u64;
    let chunk_per_worker = (batch_size / threads_u64).max(1);

    std::thread::scope(|scope| {
        for worker in 0..threads {
            let found = Arc::clone(&found);
            let winning_nonce = Arc::clone(&winning_nonce);
            let challenge = Arc::clone(&challenge_arc);
            let target = Arc::clone(&target_arc);

            scope.spawn(move || {
                let worker_u64 = worker as u64;
                let begin = start_nonce.saturating_add(worker_u64.saturating_mul(chunk_per_worker));
                let mut end = begin.saturating_add(chunk_per_worker);
                if worker + 1 == threads {
                    end = start_nonce.saturating_add(batch_size);
                }

                let mut preimage = [0u8; 40];
                preimage[..32].copy_from_slice(&challenge[..]);

                let mut nonce = begin;
                while nonce < end && !found.load(Ordering::Relaxed) {
                    if hash_meets_target_preimage(&mut preimage, nonce, &target) {
                        if !found.swap(true, Ordering::Relaxed) {
                            winning_nonce.store(nonce, Ordering::Relaxed);
                        }
                        break;
                    }
                    nonce = nonce.saturating_add(1);
                }
            });
        }
    });

    if found.load(Ordering::Relaxed) {
        Some(winning_nonce.load(Ordering::Relaxed))
    } else {
        None
    }
}
