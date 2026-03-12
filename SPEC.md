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
- 사용자는 설정 파일 생성, 토큰 발급, 대표 시세 조회를 CLI에서 바로 수행할 수 있어야 한다.
- 사용자는 아직 래핑되지 않은 API도 raw 명령으로 직접 호출할 수 있어야 한다.
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
- 국내주식 현재가 조회
- 범용 REST `GET` / `POST`
- JSON pretty/compact 출력

### Excluded
- 주문 실행
- 계좌 잔고/체결 래핑 전반
- 웹소켓 스트리밍
- 자동 재시도, rate-limit 백오프 고도화
- 설치 패키징(Homebrew, Scoop, deb/rpm 등)

## CLI Shape

예상 명령 구조:

```text
kis-trading-cli config init
kis-trading-cli config path
kis-trading-cli auth token --env demo
kis-trading-cli quote domestic-price --env real --symbol 005930
kis-trading-cli api get --env real --path /... --tr-id ...
kis-trading-cli api post --env real --path /... --tr-id ... --field key=value
```

## Architecture

### Config
- 기본 위치는 OS별 app config directory를 사용한다.
- config 파일 형식은 TOML로 한다.
- `real`, `demo` 프로필을 분리한다.
- 환경변수로 주요 값을 override 할 수 있게 한다.

### Token Cache
- OS별 cache directory에 JSON 파일로 저장한다.
- 만료 60초 전부터는 재사용하지 않는다.
- KIS 토큰 만료 문자열은 공식 샘플 기준 KST(`Asia/Seoul`) 시각으로 해석한다.

### HTTP Layer
- 공통 헤더와 인증을 캡슐화한 `KisClient`를 둔다.
- API-level 에러(`rt_cd != "0"`)는 non-zero exit로 반환한다.
- POST 요청은 필요 시 hashkey를 선발급받아 헤더에 넣을 수 있어야 한다.

## Open Questions

- 주문 API에서 hashkey가 실제 필수인 엔드포인트 범위를 문서 기준으로 다시 정리할 필요가 있다.
- 계좌번호/상품코드가 필요한 조회 API를 어떤 UX로 노출할지 후속 설계가 필요하다.
- 배포 방식(Homebrew, GitHub Releases, Windows zip/msi)은 1차 기능 안정화 후 결정한다.

