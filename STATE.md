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
  - macOS arm64 archive
  - Linux x86_64 archive
  - Windows x86_64 archive
  - GitHub Release asset upload on `v*` tags
  - Node 24 compatible `checkout` / artifact actions
  - `gh release create/upload` based release publishing
- Added a public install script:
  - root `install.sh` supports `curl | bash`
  - detects OS/architecture, resolves GitHub Release, and downloads the matching asset
  - verifies `sha256sums.txt` when available
  - re-running the script now performs install/update/no-op based on the installed binary version
  - `--check` emits a JSON install plan, and `--force` / `--allow-downgrade` control reinstall or downgrade behavior
- Added encrypted config secret storage:
  - `config set-secret` for encrypted writes
  - `config seal` for migrating plaintext config values
  - environment variables remain plaintext overrides
  - local key file path exposed through `config path`
- Added config key lifecycle commands:
  - `config key status`
  - `config key backup`
  - `config key import`
  - `config key rotate`
  - keyring format with active key + previous keys
  - backward-compatible read support for legacy single-key files
- Added structured runtime error reporting:
  - `api_error` for KIS HTTP / `rt_cd != "0"` failures
  - `program_error` for CLI/config/network/local runtime failures
  - JSON envelope with `llm_hint`, `causes`, and stable exit codes
- Added plaintext secret enforcement for config-backed secrets:
  - API/auth calls now reject plaintext `app_key`, `app_secret`, `account_no`, `hts_id`
  - `config key status` exposes `plaintext_field_count`, `plaintext_fields`, `seal_required`, and `suggested_commands`
  - `program_error.category=plaintext_secret_detected` includes self-heal commands for LLMs
- Verified `config init` writes a template to the OS-specific config directory outside the repository.
- Loaded user-provided credentials into the external config file only, not into the repository.
- Rotated the local default config key into keyring format and verified the migrated config remains usable.
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
  - raw GitHub `install.sh` is the primary bootstrap path for macOS/Linux shells
  - GitHub Actions artifacts on push, pull_request, and manual runs
  - GitHub Release assets and `sha256sums.txt` on `v*` tags
- Secret strategy:
  - config stores encrypted values for `app_key`, `app_secret`, `account_no`, `hts_id`
  - `account_product_code`, URLs, and `user_agent` stay plaintext in config
  - environment variables override config without decryption
  - plaintext sensitive config values block API/auth execution until they are sealed
  - key rotation upgrades local key handling to keyring format while keeping previous keys available for decryption
- Error strategy:
  - stderr uses structured JSON for runtime failures
  - exit code `2` means `program_error`
  - exit code `3` means `api_error`
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
- `cargo test`: passed (`15` tests)
- `cargo run -- --help`: passed
- `cargo run -- domestic-stock --help`: passed
- `cargo run -- domestic-stock inquire-balance --help`: passed
- `cargo run -- config path`: passed
- `cargo run -- config init`: passed
- `./target/release/kis-trading-cli auth token`: passed against demo credentials
- `./target/release/kis-trading-cli domestic-stock inquire-balance ...`: passed against demo credentials
- `python3 tools/render_cli_reference.py data/kis_api_manifest.json docs/CLI_REFERENCE.md`: passed
- `.github/workflows/prebuilt.yml`: YAML syntax validated locally
- `./target/release/kis-trading-cli config key rotate --compact`: passed against the local default config
- `./target/release/kis-trading-cli config key status --compact`: keyring format confirmed
- `cargo test`: passed after adding key lifecycle, error classification, and plaintext enforcement tests (`15` tests)
- `./target/release/kis-trading-cli config set-secret --field app-key`: structured `program_error` confirmed
- `./target/release/kis-trading-cli domestic-stock inquire-balance ... --afhr-flpr-yn BAD`: structured `api_error` confirmed
- `./target/release/kis-trading-cli --config <temp> config key status --compact`: plaintext fields and `seal_required` confirmed
- `./target/release/kis-trading-cli --config <temp> auth token --compact`: structured `program_error` with `plaintext_secret_detected` and remediation commands confirmed
- `bash -n ./install.sh`: passed
- `./install.sh --help`: passed
- `./install.sh --dry-run`: public repo currently returns a clear "no GitHub Release may be published yet" error
- `./install.sh` with local mock release metadata/assets: passed through download, checksum verification, extraction, and install
- `./install.sh --check` with local mock release metadata/assets: confirmed `install`, `noop`, `update`, and `downgrade_blocked` actions
- `./install.sh --allow-downgrade` with local mock release metadata/assets: downgrade install confirmed
- `.github/workflows/prebuilt.yml`: release checksum manifest step added
- `.github/workflows/prebuilt.yml`: updated to `macos-15-intel`, `macos-15`, `actions/checkout@v5`, `actions/upload-artifact@v6`, `actions/download-artifact@v7`
- `.github/workflows/prebuilt.yml`: `gh release` now uses explicit `GH_REPO` / `--repo` so the release job works without repository checkout

## Risks / Blockers

- The local machine now has Rust installed through Homebrew; if that is not desired long-term, the user may want to manage the toolchain with `rustup` later.
- 일부 주문 API는 복잡한 TR ID/파라미터 조합이 있으므로 실제 주문까지 검증하려면 추가 실계좌/모의계좌 테스트가 필요하다.
- macOS/Windows 배포의 코드 서명과 notarization은 아직 범위 밖이다.
- 실제 공개 GitHub Release 객체가 아직 없으면 `install.sh`는 의도적으로 설치를 중단한다.
- 로컬 key file 기반 암호화는 평문 저장보다 안전하지만, 동일 사용자 권한의 완전한 비밀 저장소를 대체하지는 않는다.
- key backup은 key snapshot이지 config snapshot이 아니므로, rollback에는 matching config와 함께 관리해야 한다.
- `--help`와 `--version`은 clap 기본 출력 경로를 그대로 유지하므로 JSON 오류 envelope 대상이 아니다.

## Next

- 더 많은 live smoke test를 추가한다. 특히 주문 전 조회, 해외주식 조회, 선물옵션 조회를 우선 검증한다.
- manifest 변경 시 `docs/CLI_REFERENCE.md`를 자동 재생성하는 흐름을 정리한다.
- help 출력이 길어지는 카테고리에 대해 요약/검색 명령을 추가할지 결정한다.
- key export가 필요한지, 아니면 backup/import만으로 충분한지 결정한다.
- tag/release 운영 규칙과 버전 정책을 정리한다.
