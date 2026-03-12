# kis-trading-cli

한국투자증권 Open API를 단일 네이티브 바이너리로 호출하기 위한 Rust CLI입니다.
공식 `open-trading-api` 저장소를 기준으로 API 카탈로그를 내장하고, 카테고리와 API를
CLI 명령 트리로 그대로 노출합니다.

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

### 1. 빌드

```bash
cargo build --release
```

빌드 결과:

- macOS/Linux: `./target/release/kis-trading-cli`
- Windows: `.\target\release\kis-trading-cli.exe`

명령 구조는 동일하고 실행 파일 확장자만 다릅니다.

### 2. 설정 파일 경로 확인 및 초기화

```bash
./target/release/kis-trading-cli config path
./target/release/kis-trading-cli config init
```

config와 token cache는 저장소 밖 OS 전용 경로에 생성됩니다.

### 3. config 채우기

config 파일에는 아래 값을 넣습니다.

- `profiles.real.app_key`
- `profiles.real.app_secret`
- `profiles.real.account_no`
- `profiles.real.account_product_code`
- `profiles.real.hts_id`
- `profiles.demo.app_key`
- `profiles.demo.app_secret`
- `profiles.demo.account_no`
- `profiles.demo.account_product_code`
- `profiles.demo.hts_id`

환경변수 override도 지원합니다.

- `KIS_REAL_APP_KEY`
- `KIS_REAL_APP_SECRET`
- `KIS_DEMO_APP_KEY`
- `KIS_DEMO_APP_SECRET`
- `KIS_REAL_ACCOUNT_NO`
- `KIS_DEMO_ACCOUNT_NO`

### 4. 대표 명령 실행

토큰 발급:

```bash
./target/release/kis-trading-cli auth token
```

국내주식 현재가:

```bash
./target/release/kis-trading-cli domestic-stock inquire-price \
  --fid-cond-mrkt-div-code J \
  --fid-input-iscd 005930
```

국내주식 잔고조회:

```bash
./target/release/kis-trading-cli domestic-stock inquire-balance \
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
./target/release/kis-trading-cli --help
```

특정 카테고리:

```bash
./target/release/kis-trading-cli domestic-stock --help
```

특정 API:

```bash
./target/release/kis-trading-cli domestic-stock inquire-balance --help
```

내장 카탈로그 요약/내보내기:

```bash
./target/release/kis-trading-cli catalog summary
./target/release/kis-trading-cli catalog export --compact
```

## 설계 원칙

- 공식 저장소와 포털 문서를 우선 기준으로 사용합니다.
- 비밀정보와 토큰 캐시는 저장소 안이 아니라 OS 전용 디렉터리에 둡니다.
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
