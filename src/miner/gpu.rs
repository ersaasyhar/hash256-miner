#[cfg(feature = "gpu")]
use ocl::{flags, Buffer, Kernel, ProQue};

#[cfg(feature = "gpu")]
pub struct GpuMiner {
    kernel: Kernel,
    challenge_buffer: Buffer<u8>,
    target_buffer: Buffer<u8>,
    success_buffer: Buffer<i32>,
    found_buffer: Buffer<u64>,
    cached_challenge: Option<[u8; 32]>,
    cached_target: Option<[u8; 32]>,
}

#[cfg(feature = "gpu")]
impl GpuMiner {
    pub fn new(kernel_src: &str, global_work: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let pro_que = ProQue::builder().src(kernel_src).dims(global_work).build()?;

        let queue = pro_que.queue().clone();
        let challenge_buffer = Buffer::<u8>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(32)
            .build()?;
        let target_buffer = Buffer::<u8>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(32)
            .build()?;
        let success_buffer = Buffer::<i32>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_READ_WRITE)
            .len(1)
            .build()?;
        let found_buffer = Buffer::<u64>::builder()
            .queue(queue)
            .flags(flags::MEM_WRITE_ONLY)
            .len(1)
            .build()?;

        let kernel = pro_que
            .kernel_builder("mine_keccak")
            .arg(&challenge_buffer)
            .arg(&target_buffer)
            .arg(0u64)
            .arg(&success_buffer)
            .arg(&found_buffer)
            .build()?;

        Ok(Self {
            kernel,
            challenge_buffer,
            target_buffer,
            success_buffer,
            found_buffer,
            cached_challenge: None,
            cached_target: None,
        })
    }

    pub fn mine_batch(
        &mut self,
        challenge: [u8; 32],
        target_be: [u8; 32],
        start_nonce: u64,
    ) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        if self.cached_challenge != Some(challenge) {
            self.challenge_buffer.write(&challenge[..]).enq()?;
            self.cached_challenge = Some(challenge);
        }
        if self.cached_target != Some(target_be) {
            self.target_buffer.write(&target_be[..]).enq()?;
            self.cached_target = Some(target_be);
        }
        self.success_buffer.write(&[0i32][..]).enq()?;

        self.kernel.set_arg(2, start_nonce)?;

        // SAFETY: Kernel args and dimensions are fully initialized and match kernel signature.
        unsafe { self.kernel.enq()?; }

        let mut success = [0i32; 1];
        self.success_buffer.read(&mut success[..]).enq()?;
        if success[0] == 1 {
            let mut found = [0u64; 1];
            self.found_buffer.read(&mut found[..]).enq()?;
            Ok(Some(found[0]))
        } else {
            Ok(None)
        }
    }
}
