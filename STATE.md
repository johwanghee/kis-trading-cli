# STATE.md

## Snapshot

- Date: 2026-03-12
- Status: phase-1 scaffold implemented
- Workspace started as an almost-empty git repository.
- Rust toolchain was not available on PATH at start, then installed via Homebrew for local verification.

## Completed

- Collected official KIS Open API sample references from `koreainvestment/open-trading-api`.
- Confirmed REST base URLs, OAuth endpoint, websocket approval endpoint, common headers, and one representative quote endpoint.
- Defined phase-1 CLI scope in `SPEC.md`.
- Added root project documents: `AGENTS.md`, `SPEC.md`, `STATE.md`.
- Added Rust crate with:
  - cross-platform config path resolution
  - config template generation
  - OAuth token issuance and cache
  - file lock around token refresh/cache updates for concurrent test safety
  - domestic stock current price command
  - generic REST `GET` / `POST`
  - optional hashkey support for POST
  - JSON path selection for CLI output
- Verified `config init` writes a template to the OS-specific config directory outside the repository.

## Active Decisions

- Language/runtime: Rust native binary
- HTTP client: `reqwest` + `rustls`
- Config format: TOML
- Config storage: OS-specific app config directory
- Token cache storage: OS-specific app cache directory
- Initial supported feature set:
  - config template generation
  - OAuth token issuance/cache
  - domestic stock current price quote
  - generic REST GET/POST

## Verification

- `cargo fmt -- --check`: passed
- `cargo check`: passed
- `cargo test`: passed (`6` tests)
- `cargo run -- --help`: passed
- `cargo run -- config path`: passed
- `cargo run -- config init`: passed

## Risks / Blockers

- Live API verification is blocked until valid KIS app credentials are configured.
- The local machine now has Rust installed through Homebrew; if that is not desired long-term, the user may want to manage the toolchain with `rustup` later.

## Next

- Fill generated config with KIS credentials and run live token/quote commands.
- Decide the next wrapped endpoints after `quote domestic-price` (likely balance, order, or overseas quote).
- Add release packaging strategy once the command surface stabilizes.
