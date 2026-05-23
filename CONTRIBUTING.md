# Contributing to Episode Transcribe

Thank you for considering a contribution. This project is a Rust CLI for batch-transcribing tabletop episode MP4s.

## Before you start

1. Check existing [issues](https://github.com/your-org/transcribe/issues) for duplicates or planned work.
2. For large changes, open an issue first so we can agree on approach.
3. Read [README.md](README.md) for setup and [models/README.md](models/README.md) for model downloads.

## Development setup

```bash
git clone https://github.com/your-org/transcribe.git
cd transcribe
```

Install [Rust](https://rustup.rs/) (stable, 1.75+) and ensure [ffmpeg](https://ffmpeg.org/) is on your `PATH`.

CPU-only builds avoid CUDA requirements:

```bash
cargo build --no-default-features
cargo test --no-default-features
cargo run --no-default-features -- help
```

Optional: download test models (see [models/README.md](models/README.md)) for coverage and e2e tests:

```bash
# Coverage (requires cargo-llvm-cov: cargo install cargo-llvm-cov)
./scripts/coverage.sh

# Full e2e (ignored tests)
TRANSCRIBE_E2E=1 cargo test --release --no-default-features -- --ignored
```

## Before you push

Run the same fast checks CI runs on every push, **before** `git push` — do not wait for CI to fail on formatting.

**Recommended (full local parity with CI lint + test):**

```bash
./scripts/check.sh
```

On Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

**Minimum before push** (matches CI `lint` job — fast):

```bash
cargo fmt --all -- --check
cargo clippy --locked --no-default-features -- -D warnings
```

Or use the pre-push script directly:

```bash
./scripts/pre-push.sh
```

**Optional: install a pre-push hook** (opt-in, repo-local only):

```bash
./scripts/install-git-hooks.sh
```

This sets `git config core.hooksPath .githooks` for **this repository only** (not global). After install, `git push` runs `./scripts/pre-push.sh` automatically. Skip once with `git push --no-verify`.

## Code style

- Run `cargo fmt` before committing; CI uses `cargo fmt --all -- --check`.
- Keep `cargo clippy --locked --no-default-features -- -D warnings` clean.
- Match existing module layout and naming in `src/`.
- Prefer focused changes; avoid drive-by refactors.

## Submitting changes

1. Fork the repository on GitHub (web UI).
2. Add your fork as a remote and create a branch:

   ```bash
   git remote add fork git@github.com:YOUR_USERNAME/transcribe.git
   git checkout -b short-descriptive-name
   ```

3. Make your changes and verify locally (see [Before you push](#before-you-push)):

   ```bash
   ./scripts/check.sh
   ```

4. Commit with a clear message:

   ```bash
   git add -p
   git commit -m "Brief summary of why this change is needed"
   ```

5. Push and open a pull request:

   ```bash
   git push -u fork short-descriptive-name
   ```

   On GitHub, use **Compare & pull request** (or **New pull request** → choose your fork and branch).

6. Fill out the pull request template. CI must pass before merge.

## Reporting bugs and requesting features

Use the issue templates on GitHub (**Issues → New issue**). Include OS, Rust version (`rustc --version`), and steps to reproduce for bugs.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be respectful and constructive.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
