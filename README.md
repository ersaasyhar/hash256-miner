# hash256-miner

Rust miner for [hash256.org](https://hash256.org/mine) with:
- GPU mode (OpenCL, feature-gated)
- CPU fallback mode
- Auto state refresh (epoch/difficulty/challenge)
- Optional on-chain submission (`mine(uint64)`)

## 1. Requirements (Windows)

- Windows 10/11 x64
- Rust (stable, MSVC toolchain)
- Visual Studio 2022 Build Tools (C++ tools)
- Git
- For GPU mode: OpenCL development library (`OpenCL.lib`)

## 2. Install Rust

Open PowerShell and install Rust:

```powershell
winget install Rustlang.Rustup
```

Then ensure target is MSVC:

```powershell
rustup default stable-x86_64-pc-windows-msvc
rustc -V
cargo -V
```

## 3. Clone project

```powershell
git clone https://github.com/ersaasyhar/hash256-miner.git
cd hash256-miner
```

## 4. Configure environment

Copy example env:

```powershell
Copy-Item .env.example .env.hehe
```

Edit `.env` and set at least:

```dotenv
PRIVATE_KEY=0xYOUR_PRIVATE_KEY
```

Recommended baseline:

```dotenv
RPC_URL=https://ethereum.publicnode.com
CONTRACT_ADDRESS=0xAC7b5d06fa1e77D08aea40d46cB7C5923A87A0cc
MODE=auto
BATCH_SIZE=16000000
THREADS=16
REFRESH_EVERY_BATCHES=10
START_NONCE=0
SUBMIT=1
```  

or  

```
RPC_URL=https://ethereum.publicnode.com
CONTRACT_ADDRESS=0xAC7b5d06fa1e77D08aea40d46cB7C5923A87A0cc
MODE=gpu
BATCH_SIZE=32000000
THREADS=16
REFRESH_EVERY_BATCHES=50
START_NONCE=0
SUBMIT=1
```

Notes:
- `MODE=auto` tries GPU first then CPU fallback.
- `THREADS` is used by CPU mode/fallback only.
- Leave `EPOCH` and `DIFFICULTY_TARGET` unset for auto-detection.

## 5. CPU-only run (works without OpenCL SDK)

```powershell
cargo run --release
```

## 6. GPU setup (OpenCL.lib) on Windows

If GPU build fails with `LNK1181 cannot open input file OpenCL.lib`, install OpenCL dev libs via `vcpkg`.

### 6.1 Install vcpkg

In Git Bash or PowerShell:

```bash
git clone https://github.com/microsoft/vcpkg.git ~/vcpkg
cd ~/vcpkg
./bootstrap-vcpkg.bat
./vcpkg.exe install opencl:x64-windows
```

### 6.2 Verify `OpenCL.lib`

```powershell
Get-ChildItem $HOME\vcpkg\installed\x64-windows\lib\OpenCL.lib
```

### 6.3 Configure Cargo OpenCL path

Create `.cargo/config.toml` from `.cargo/config.toml.example` and set your local vcpkg path.

## 7. GPU run

```powershell
cargo run --release --features gpu
```

To force GPU (no auto fallback):

```powershell
$env:MODE='gpu'
cargo run --release --features gpu
```

## 8. Runtime output checks

At startup, miner prints:
- Wallet address
- Wallet ETH balance
- Wallet HASH balance (via `balanceOf(address)`)
- Epoch, target, challenge
- Mode (`gpu` or `cpu`)

## 9. Performance tuning

For smoother GPU utilization:

```dotenv
MODE=gpu
SUBMIT=0
BATCH_SIZE=32000000
REFRESH_EVERY_BATCHES=50
```

Try `BATCH_SIZE=64000000` if stable.

Benchmark mode:
- keep `SUBMIT=0` (no transaction overhead)

## 10. Submission mode

Enable tx submission after benchmarking:

```dotenv
SUBMIT=1
```

Miner will preflight with `eth_call` before sending tx; if preflight reverts, it refreshes state and continues.

## 11. Common issues

### `LNK1181: cannot open input file 'OpenCL.lib'`

OpenCL runtime exists, but dev import lib is missing/not found. Install via `vcpkg` and ensure `.cargo/config.toml` path matches your machine.

### `Blocking waiting for file lock on build directory`

Another cargo/rustc process is running. Wait or stop stale processes, then rerun.

### Revert on submit

State may have rotated (epoch/target). Miner auto-refreshes; keep `EPOCH`/`DIFFICULTY_TARGET` unset unless intentionally pinned.

## 12. Security

- Never commit real private keys.
- Keep `.env` private.
- Use dedicated wallet for mining.
