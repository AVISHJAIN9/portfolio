# What Only You Can Do (User Action Items)

This document tracks items that require external user interaction, manual terminal setup, hardware configuration, or personal repository credentials.

---

### 1. Fuzz Toolchain Installation
- **Command**:
  ```bash
  rustup toolchain install nightly && cargo install cargo-fuzz
  ```
- **Check**:
  Verify that `~/.cargo/bin` is in your shell `PATH` so `cargo +nightly fuzz --version` runs properly.

---

### 2. Python Dependencies & Local Network Permissions
- **Command**:
  ```bash
  /Users/avishjain/venv/bin/pip install aioquic duckdb pyarrow openpyxl pytest
  ```
- **Localhost Networking**:
  If macOS displays an "Allow incoming network connections" popup for UDP on `127.0.0.1`, click **Allow**.

---

### 3. Disk Space for TPC-H SF1 Benchmarks
- Free up **5 GB or more** of disk space for the TPC-H SF1 dataset generation. DuckDB generates this locally in-memory/temp files; no external dataset download is required.

---

### 4. Benchmark Execution Conditions
- Plug in the power charger.
- Quit resource-heavy applications (browsers, simulators, video editors).
- Do not interact with the laptop during Criterion or TPC-H benchmark runs to minimize CPU frequency throttling and OS scheduling jitter.

---

### 5. Review Generated Status Workbooks & ADRs
- Open the three generated master status workbooks in Microsoft Excel / LibreOffice:
  - `evidence/workbooks/04_Columnar_Query_Engine_ULTIMATE_145_Master_STATUS.xlsx`
  - `evidence/workbooks/05_Mini_QUIC_ULTIMATE_145_Master_STATUS.xlsx`
  - `evidence/workbooks/06_LSM_KV_ENGINE_ULTIMATE_145_Master_STATUS.xlsx`
- Spot-check 5 verification ticks against test functions in `projects/*/backend/tests/`.
- Review `docs/DECISIONS.md` (ADR-001 through ADR-025).

---

### 6. GitHub Repository Setup, Actions Execution & Run URL
- Authenticate GitHub CLI:
  ```bash
  gh auth login
  ```
- Create remote repo and push:
  ```bash
  git remote add origin git@github.com:<username>/portfolio.git
  git branch -M main
  git push -u origin main
  ```
- Enable GitHub Actions on the repository:
  1. Navigate to `https://github.com/<username>/portfolio/actions`.
  2. Trigger workflow runs for `.github/workflows/ci.yml` and `.github/workflows/bench.yml`.
  3. Once green, paste the successful run URL below:
     - **CI Run URL**: `[PASTE_GREEN_CI_RUN_URL_HERE]`
     - **Benchmark Run URL**: `[PASTE_GREEN_BENCH_RUN_URL_HERE]`

---

### 7. Slot 04 CI/CD Pipeline Configuration (04-133)
- **Feature**: `04-133` "CI/CD pipeline"
- **Status**: Specified (Manual verification required on GitHub Actions)
- **Action**: Workflow file `.github/workflows/ci.yml` is committed. User must push the repository and verify the live run on GitHub Actions runner.

---

### 8. Slot 05 Benchmark CI Jobs (05-142)
- **Feature**: `05-142` "Benchmark CI jobs"
- **Status**: Specified (Manual verification required on GitHub Actions)
- **Action**: Matrix workflow file `.github/workflows/bench.yml` is committed. User must push the repository and verify the benchmark matrix run on GitHub Actions runner.
