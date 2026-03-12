# STATE.md

## Snapshot

- Date: 2026-03-12
- Status: manifest-driven CLI implemented
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
  - embedded official API manifest (`166` APIs, `8` categories)
  - category/API command tree generated from the manifest
  - generic REST executor driven by manifest metadata
  - special TR ID resolvers for the small set of multi-branch order APIs
  - optional hashkey support for POST
- Added documentation split for different readers:
  - `README.md` for human quick start and project overview
  - `docs/LLM_GUIDE.md` for LLM/agent execution rules
  - `docs/CLI_REFERENCE.md` as a generated full command reference
  - `tools/render_cli_reference.py` to regenerate the reference from the embedded manifest
- Added GitHub Actions workflow for prebuilt binaries:
  - macOS x86_64 archive
  - Linux x86_64 archive
  - Windows x86_64 archive
  - GitHub Release asset upload on `v*` tags
- Verified `config init` writes a template to the OS-specific config directory outside the repository.
- Loaded user-provided credentials into the external config file only, not into the repository.
- Verified live calls against demo environment using the new command tree:
  - `auth token`
  - `domestic-stock inquire-balance`

## Active Decisions

- Language/runtime: Rust native binary
- HTTP client: `reqwest` + `rustls`
- Config format: TOML
- Config storage: OS-specific app config directory
- Token cache storage: OS-specific app cache directory
- Command surface source: generated manifest from official `MCP` config + `examples_llm`
- Documentation strategy:
  - README for people
  - dedicated LLM guide for task mapping and execution rules
  - generated command reference for full catalog coverage
- Distribution strategy:
  - prebuilt binaries are the primary user flow
  - GitHub Actions artifacts on push, pull_request, and manual runs
  - GitHub Release assets on `v*` tags
- Current visible command model:
  - `config`
  - `catalog`
  - `auth`
  - `domestic-stock`
  - `domestic-bond`
  - `domestic-futureoption`
  - `overseas-stock`
  - `overseas-futureoption`
  - `etfetn`
  - `elw`

## Verification

- `cargo fmt -- --check`: passed
- `cargo check`: passed
- `cargo test`: passed (`6` tests)
- `cargo run -- --help`: passed
- `cargo run -- domestic-stock --help`: passed
- `cargo run -- domestic-stock inquire-balance --help`: passed
- `cargo run -- config path`: passed
- `cargo run -- config init`: passed
- `./target/release/kis-trading-cli auth token`: passed against demo credentials
- `./target/release/kis-trading-cli domestic-stock inquire-balance ...`: passed against demo credentials
- `python3 tools/render_cli_reference.py data/kis_api_manifest.json docs/CLI_REFERENCE.md`: passed
- `.github/workflows/prebuilt.yml`: YAML syntax validated locally

## Risks / Blockers

- The local machine now has Rust installed through Homebrew; if that is not desired long-term, the user may want to manage the toolchain with `rustup` later.
- 일부 주문 API는 복잡한 TR ID/파라미터 조합이 있으므로 실제 주문까지 검증하려면 추가 실계좌/모의계좌 테스트가 필요하다.
- macOS/Windows 배포의 코드 서명과 notarization은 아직 범위 밖이다.

## Next

- 더 많은 live smoke test를 추가한다. 특히 주문 전 조회, 해외주식 조회, 선물옵션 조회를 우선 검증한다.
- manifest 변경 시 `docs/CLI_REFERENCE.md`를 자동 재생성하는 흐름을 정리한다.
- help 출력이 길어지는 카테고리에 대해 요약/검색 명령을 추가할지 결정한다.
- tag/release 운영 규칙과 버전 정책을 정리한다.
