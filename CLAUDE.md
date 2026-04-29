# CLAUDE.md

## Goal
- Build a cross-platform Rust CLI for Korea Investment & Securities Open API.
- The binary should run with the same command shape on macOS, Linux, and Windows.

## Source Of Truth
- Official sample repo: https://github.com/koreainvestment/open-trading-api
- Prefer the official repository and portal documentation over blog posts or third-party wrappers.
- When behavior differs between docs and examples, record the discrepancy in `STATE.md` before changing code.

## Current MVP Boundary
- `config init`: write a local config template outside the repo.
- Embed the official KIS API catalog in the binary.
- Expose official API categories and functions as CLI commands so `--help` reveals the command surface.
- Use a manifest-driven executor for the official REST APIs instead of raw URL-first UX.
- Keep auth/token flows and account-backed APIs usable through local config defaults.

## Engineering Rules
- Do not commit real app keys, app secrets, account numbers, or token cache files.
- Keep output JSON-first so the CLI composes well with `jq`, PowerShell, and other shells.
- Favor `reqwest` with `rustls` to avoid platform-specific OpenSSL packaging issues.
- Keep config and token cache in OS-specific app directories, not in the repository.
- Update `SPEC.md` when scope changes and `STATE.md` after meaningful implementation progress.

## Workflow Preference
- After completing a logical work unit, create a git commit and push it to the current remote branch unless the user says otherwise.

## Verification
- Preferred checks: `cargo fmt`, `cargo test`, `cargo build`.
- If Rust toolchain is unavailable, note the exact blocker in `STATE.md` and the final response.

## Release Process
- GitHub Actions (`.github/workflows/prebuilt.yml`) builds binaries on every push to `main` and on `v*` tag pushes.
- A GitHub Release (with prebuilt archives and `sha256sums.txt`) is only published when a `v*` tag is pushed.
- Steps to cut a release:
  1. Bump `version` in `Cargo.toml` (e.g. `1.0.3` → `1.0.4`).
  2. `cargo update` — Cargo.lock picks up the new version automatically.
  3. `git add Cargo.toml Cargo.lock && git commit -m "chore: X.Y.Z 버전 반영"`
  4. `git tag vX.Y.Z && git push origin main --tags`
- After the tag push, GitHub Actions creates the release automatically (~5–10 min).
- Do not push a tag without first updating `Cargo.toml`; version and tag must match.
