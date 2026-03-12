# SPEC.md

## Objective

한국투자증권 Open API를 Python 샘플 없이 직접 호출할 수 있는 설치형 Rust CLI를 만든다.  
초기 목표는 조회 중심의 안정적인 CLI를 먼저 제공하고, 이후 주문/웹소켓으로 확장 가능한 구조를 확보하는 것이다.

## Research Snapshot

기준 시점: 2026-03-12  
참조 저장소: `koreainvestment/open-trading-api` `main` 브랜치 (`2eb8aa1408fdfa08bcef033576815817269c4e39` 확인)

확인한 주요 공식 자료:
- 저장소 `README.md`
- 저장소 `kis_devlp.yaml`
- 저장소 `examples_llm/auth/auth_token/auth_token.py`
- 저장소 `examples_llm/domestic_stock/inquire_price/inquire_price.py`
- 저장소 `examples_user/kis_auth.py`

공식 샘플에서 확인한 핵심 정보:
- 실전 REST base URL: `https://openapi.koreainvestment.com:9443`
- 모의 REST base URL: `https://openapivts.koreainvestment.com:29443`
- OAuth 접근토큰 발급: `POST /oauth2/tokenP`
- 웹소켓 접속키 발급: `POST /oauth2/Approval`
- 국내주식 현재가 조회: `GET /uapi/domestic-stock/v1/quotations/inquire-price`
- 국내주식 현재가 TR ID: `FHKST01010100`
- 공통 헤더: `authorization`, `appkey`, `appsecret`, `tr_id`, `custtype`, `tr_cont`
- 공식 Python helper는 모의투자에서 `T/J/C`로 시작하는 TR ID를 `V` prefix로 치환한다
- README에는 토큰 재발급 관련 주의사항으로 `1분당 1회 발급`이 명시되어 있다

## Product Requirements

### Functional
- 사용자는 단일 바이너리로 KIS REST API를 호출할 수 있어야 한다.
- 사용자는 실전/모의 환경을 명시적으로 전환할 수 있어야 한다.
- 사용자는 설정 파일 생성, 토큰 발급, 계좌조회 등 대표 기능을 CLI에서 바로 수행할 수 있어야 한다.
- 사용자는 공식 KIS API 기능이 카테고리별 CLI 서브커맨드로 노출된 도움말을 볼 수 있어야 한다.
- LLM은 CLI help만이 아니라 전용 문서(`docs/LLM_GUIDE.md`, `docs/CLI_REFERENCE.md`)와 embedded manifest를 통해 사용 가능한 기능과 파라미터를 빠르게 파악할 수 있어야 한다.
- 출력은 기본적으로 JSON이어야 한다.

### Non-Functional
- macOS, Linux, Windows에서 동일한 명령 구조를 유지한다.
- Python 런타임에 의존하지 않는다.
- 비밀정보는 저장소 바깥 경로에 저장한다.
- TLS는 `rustls` 기반으로 구성해 배포 난이도를 낮춘다.

## Phase 1 Scope

### Included
- Rust binary crate 초기화
- OS별 config/cache 경로 사용
- config template 생성
- OAuth 토큰 발급 및 캐시
- 공식 MCP config 기반 API manifest 생성
- 카테고리별 동적 CLI 도움말/명령 트리
- 사람용 README와 LLM 전용 운영 문서 분리
- manifest 기반 전체 CLI reference 생성
- GitHub Actions 기반 3개 OS prebuilt binary 빌드와 release asset 업로드
- config 비밀값 암복호화 저장과 plaintext-to-encrypted migration command
- manifest 기반 REST executor
- 대표 계좌조회 실호출 검증
- JSON pretty/compact 출력

### Excluded
- 웹소켓 스트리밍
- 자동 재시도, rate-limit 백오프 고도화
- 설치 패키징(Homebrew, Scoop, deb/rpm 등)
- 코드 서명 및 notarization

## CLI Shape

예상 명령 구조:

```text
kis-trading-cli config init
kis-trading-cli config path
kis-trading-cli catalog summary
kis-trading-cli auth token
kis-trading-cli domestic-stock --help
kis-trading-cli domestic-stock inquire-price --fid-cond-mrkt-div-code J --fid-input-iscd 005930
kis-trading-cli domestic-stock inquire-balance --afhr-flpr-yn N --inqr-dvsn 01 --unpr-dvsn 01 --fund-sttl-icld-yn N --fncg-amt-auto-rdpt-yn N --prcs-dvsn 00
```

## Architecture

### Command Surface
- 공식 저장소 `MCP/Kis Trading MCP/configs/*.json`를 기반으로 API 카탈로그를 생성한다.
- `examples_llm` Python 소스에서 `http_method`, `tr_id`, 요청 필드, 연속조회 컨텍스트를 추출해 실행 manifest로 합친다.
- 바이너리는 이 embedded manifest를 읽어 카테고리/기능별 CLI help를 동적으로 구성한다.
- 별도 문서 계층은 다음처럼 유지한다:
  - `README.md`: 사람용 소개와 빠른 시작
  - `docs/LLM_GUIDE.md`: LLM용 호출 규칙과 작업 매핑
  - `docs/CLI_REFERENCE.md`: manifest에서 생성한 전체 명령 레퍼런스
  - `data/kis_api_manifest.json`: 기계 판독용 원본

### Distribution
- 기본 사용 흐름은 prebuilt 바이너리 다운로드를 우선으로 한다.
- GitHub Actions는 macOS, Linux, Windows용 release binary를 매 빌드마다 생성한다.
- `v*` 태그에서는 GitHub Release 자산으로 같은 바이너리를 업로드한다.
- Homebrew, Scoop, apt/rpm 같은 패키지 매니저 배포는 아직 범위 밖이다.

### Config
- 기본 위치는 OS별 app config directory를 사용한다.
- config 파일 형식은 TOML로 한다.
- `real`, `demo` 프로필을 분리한다.
- 환경변수로 주요 값을 override 할 수 있게 한다.
- 환경변수는 평문 입력을 그대로 사용한다.
- config의 민감값(`app_key`, `app_secret`, `account_no`, `hts_id`)은 로컬 암호화 키로 저장/복호화할 수 있어야 한다.
- 기존 평문 config를 암호문으로 바꾸는 migration command를 제공한다.

### Token Cache
- OS별 cache directory에 JSON 파일로 저장한다.
- 만료 60초 전부터는 재사용하지 않는다.
- KIS 토큰 만료 문자열은 공식 샘플 기준 KST(`Asia/Seoul`) 시각으로 해석한다.

### Secret Storage
- config 비밀값은 `enc:kis:v1:` prefix의 암호문 문자열로 저장한다.
- 암호화 키는 OS 전용 앱 디렉터리에 저장하며, custom config path를 쓰면 그 config 파일 옆 key file을 사용한다.
- 같은 값이 환경변수와 config에 동시에 있으면 환경변수가 우선한다.
- 이 구조는 주로 평문 노출과 실수로 인한 유출을 줄이기 위한 것이며, 동일 사용자 권한의 완전한 격리는 범위 밖이다.

### HTTP Layer
- 공통 헤더와 인증을 캡슐화한 `KisClient`를 둔다.
- API-level 에러(`rt_cd != "0"`)는 non-zero exit로 반환한다.
- POST 요청은 필요 시 hashkey를 선발급받아 헤더에 넣을 수 있어야 한다.
- 연속조회 API는 `tr_cont`와 `CTX_AREA_*` 컨텍스트를 manifest 정보로 자동 처리한다.

### TR ID Resolution
- 대부분 API는 manifest에 추출된 상수 또는 `real`/`demo` TR ID로 처리한다.
- 일부 복잡한 주문 API는 입력 파라미터 기반 special resolver를 둔다.

## Open Questions

- 주문 API에서 hashkey가 실제 필수인 엔드포인트 범위를 문서 기준으로 다시 정리할 필요가 있다.
- 카테고리 도움말이 너무 길어지는 문제는 문서 분리로 1차 완화했지만, CLI 자체에도 요약/검색 UX를 추가할지 검토가 필요하다.
- 코드 서명과 notarization을 어떤 시점에 도입할지 결정이 필요하다.
