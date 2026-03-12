# kis-trading-cli

한국투자증권 Open API를 단일 네이티브 바이너리로 호출하기 위한 Rust CLI입니다.
공식 `open-trading-api` 저장소를 기준으로 API 카탈로그를 내장하고, 카테고리와 API를
CLI 명령 트리로 그대로 노출합니다.

## 고지사항

- 이 프로젝트는 한국투자증권의 공식 지원 도구가 아닙니다.
- 이 프로젝트의 사용으로 발생하는 결과와 책임은 사용자에게 있습니다.
- 한국투자증권 Open API 자체의 정책, 동작, 응답, 장애에 관한 이슈는 이 저장소에서 받지 않습니다.
- API 스펙 변경이나 정책 문의는 한국투자증권 공식 문서와 공식 지원 채널을 우선 확인해야 합니다.

## 문서 구성

- 사람용 개요와 빠른 시작: `README.md`
- LLM/에이전트용 사용 규칙: `docs/LLM_GUIDE.md`
- 전체 명령 레퍼런스: `docs/CLI_REFERENCE.md`
- 기계가 읽는 원본 manifest: `data/kis_api_manifest.json`
- 저장소 작업 규칙: `AGENTS.md`

## 현재 범위

- 단일 Rust 바이너리
- 실전/모의 환경 전환
- OS별 외부 config/cache 경로 사용
- OAuth 토큰 발급과 캐시
- 공식 API 카탈로그 내장
- 카테고리별/기능별 CLI help
- manifest 기반 REST 실행

현재는 REST API 중심이며 웹소켓 스트리밍은 아직 포함하지 않습니다.

## 빠른 시작

### 1. 바이너리 받기

권장 방식은 GitHub Releases 또는 GitHub Actions artifacts에서 OS별 prebuilt 바이너리를
받아 사용하는 것입니다.

제공 대상:

- macOS x86_64: `kis-trading-cli-macos-x86_64.tar.gz`
- Linux x86_64: `kis-trading-cli-linux-x86_64.tar.gz`
- Windows x86_64: `kis-trading-cli-windows-x86_64.zip`

압축을 푼 뒤 실행 파일을 `PATH`에 두면 아래 예제를 그대로 사용할 수 있습니다.
현재 디렉터리에서 바로 실행할 때만 OS에 따라 다음처럼 앞에 경로를 붙이면 됩니다.

- macOS/Linux: `./kis-trading-cli`
- Windows: `.\kis-trading-cli.exe`

### 2. 설정 파일 경로 확인 및 초기화

```bash
kis-trading-cli config path
kis-trading-cli config init
```

config와 token cache는 저장소 밖 OS 전용 경로에 생성됩니다.
config 비밀값 암호화에 쓰는 로컬 키 파일도 함께 사용됩니다.

### 3. config 채우기

기본 원칙은 이렇습니다.

- 비밀값은 `config set-secret`으로 넣어 config 파일에 암호문으로 저장
- 이미 config에 평문이 있다면 `config seal`로 일괄 암호화
- 환경변수는 자동화/CI 용도로 평문 override 유지
- 키 파일 운영은 `config key ...` 명령으로 처리

config 파일에서 직접 채워도 되는 값:

- `user_agent`
- `profiles.real.base_url`
- `profiles.real.websocket_url`
- `profiles.real.account_product_code`
- `profiles.demo.base_url`
- `profiles.demo.websocket_url`
- `profiles.demo.account_product_code`

암호화 저장 권장 값:

- `profiles.real.app_key`
- `profiles.real.app_secret`
- `profiles.real.account_no`
- `profiles.real.hts_id`
- `profiles.demo.app_key`
- `profiles.demo.app_secret`
- `profiles.demo.account_no`
- `profiles.demo.hts_id`

예시:

```bash
kis-trading-cli config set-secret --profile real --field app-key --stdin
kis-trading-cli config set-secret --profile real --field app-secret --stdin
kis-trading-cli config set-secret --profile real --field account-no --stdin
kis-trading-cli config set-secret --profile real --field hts-id --stdin
```

이미 평문으로 들어간 값을 한 번에 암호화:

```bash
kis-trading-cli config seal
```

키 상태/백업/회전/복원:

```bash
kis-trading-cli config key status
kis-trading-cli config key backup
kis-trading-cli config key rotate
kis-trading-cli config key import --input /path/to/config.key.backup
```

권장 순서:

1. `config key status`로 현재 key 상태 확인
2. 위험 작업 전 `config key backup`
3. 정기 교체가 필요하면 `config key rotate`
4. 다른 머신이나 같은 상태의 config를 복구할 때만 `config key import`

주의:

- `config key rotate`는 현재 key를 백업한 뒤 새 active key를 만들고 config 비밀값을 재암호화합니다.
- rotate 후 생성되는 backup은 "회전 전 config 상태"와 짝이 맞는 key입니다.
- `config key import`는 현재 config를 실제로 복호화할 수 있는 key만 받아들입니다.

환경변수 override도 지원합니다.

- `KIS_REAL_APP_KEY`
- `KIS_REAL_APP_SECRET`
- `KIS_DEMO_APP_KEY`
- `KIS_DEMO_APP_SECRET`
- `KIS_REAL_ACCOUNT_NO`
- `KIS_DEMO_ACCOUNT_NO`

환경변수는 복호화 없이 그대로 사용되며, 같은 값이 config에도 있으면 환경변수가 우선합니다.

### 4. 대표 명령 실행

토큰 발급:

```bash
kis-trading-cli auth token
```

국내주식 현재가:

```bash
kis-trading-cli domestic-stock inquire-price \
  --fid-cond-mrkt-div-code J \
  --fid-input-iscd 005930
```

국내주식 잔고조회:

```bash
kis-trading-cli domestic-stock inquire-balance \
  --afhr-flpr-yn N \
  --inqr-dvsn 01 \
  --unpr-dvsn 01 \
  --fund-sttl-icld-yn N \
  --fncg-amt-auto-rdpt-yn N \
  --prcs-dvsn 00
```

## 명령 탐색

최상위 카테고리:

```bash
kis-trading-cli --help
```

특정 카테고리:

```bash
kis-trading-cli domestic-stock --help
```

특정 API:

```bash
kis-trading-cli domestic-stock inquire-balance --help
```

내장 카탈로그 요약/내보내기:

```bash
kis-trading-cli catalog summary
kis-trading-cli catalog export --compact
```

## Prebuilt 빌드

GitHub Actions는 다음 3종 prebuilt 산출물을 만듭니다.

- `macos-13`에서 빌드한 `kis-trading-cli-macos-x86_64.tar.gz`
- `ubuntu-22.04`에서 빌드한 `kis-trading-cli-linux-x86_64.tar.gz`
- `windows-2022`에서 빌드한 `kis-trading-cli-windows-x86_64.zip`

동작 방식:

- `push`, `pull_request`, `workflow_dispatch` 때마다 3종 빌드를 수행합니다.
- 각 빌드 산출물은 GitHub Actions artifact로 업로드됩니다.
- `v*` 형식 태그를 push하면 같은 산출물을 GitHub Release 자산으로도 업로드합니다.

## 소스에서 직접 빌드하기

prebuilt 바이너리 대신 로컬에서 직접 빌드하려면 아래를 사용합니다.

```bash
cargo build --release
```

빌드 결과:

- macOS/Linux: `./target/release/kis-trading-cli`
- Windows: `.\target\release\kis-trading-cli.exe`

## 설계 원칙

- 공식 저장소와 포털 문서를 우선 기준으로 사용합니다.
- 비밀정보와 토큰 캐시는 저장소 안이 아니라 OS 전용 디렉터리에 둡니다.
- config 파일의 민감값은 로컬 키 파일로 암호화 저장하고, 환경변수는 평문 override로 유지합니다.
- key rotation은 keyring 방식으로 처리해 이전 key를 함께 보관하며, config는 새 active key로 다시 암호화합니다.
- 출력은 JSON 우선으로 유지해 `jq`, PowerShell, 다른 에이전트에서 조합하기 쉽게 합니다.
- TLS는 `reqwest` + `rustls` 기반으로 구성합니다.

## LLM 친화적 사용

LLM이 이 CLI를 사용할 때는 아래 순서를 권장합니다.

1. `docs/LLM_GUIDE.md`를 읽어 호출 규칙과 기본 가정을 파악합니다.
2. `docs/CLI_REFERENCE.md`에서 카테고리와 API 이름을 찾습니다.
3. 필요한 경우 `kis-trading-cli <category> <api> --help`로 파라미터를 확인합니다.
4. 더 엄밀한 자동화가 필요하면 `data/kis_api_manifest.json`을 직접 읽습니다.

## 레퍼런스 재생성

전체 CLI 레퍼런스는 아래 스크립트로 재생성합니다.

```bash
python3 tools/render_cli_reference.py data/kis_api_manifest.json docs/CLI_REFERENCE.md
```
