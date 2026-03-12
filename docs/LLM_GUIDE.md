# LLM Guide

이 문서는 사람보다 LLM/에이전트가 빠르게 읽고 `kis-trading-cli`를 호출할 수 있게
구성한 운영 가이드입니다.

## 우선 읽을 순서

1. 이 문서 `docs/LLM_GUIDE.md`
2. 전체 명령 목록 `docs/CLI_REFERENCE.md`
3. 실제 파라미터 help `kis-trading-cli <category> <api> --help`
4. 기계가 읽는 원본 `data/kis_api_manifest.json`

## 명령 문법

기본 형태:

```text
kis-trading-cli [GLOBAL OPTIONS] <category> <api> [api flags...]
```

예외적인 최상위 그룹:

```text
kis-trading-cli config <subcommand>
kis-trading-cli catalog <subcommand>
```

## 전역 옵션

- `--env <demo|real>`
  - 기본값은 `demo`
  - 모든 API 카테고리와 `auth`에 공통 적용됩니다.
- `--config <PATH>`
  - OS 기본 config 대신 특정 TOML 파일을 사용합니다.
- `--compact`
  - 출력 JSON을 한 줄로 압축합니다.

## 설정과 자동 주입 규칙

이 CLI는 일부 파라미터를 help에서 숨기고 config 또는 환경변수에서 자동으로 채웁니다.

로컬 config의 민감값은 암호문으로 저장할 수 있고, 환경변수는 평문 override로 그대로 사용합니다.

자동 주입되는 대표 필드:

- `appkey`
- `appsecret`
- `secretkey`
- `cano`
- `acnt_prdt_cd`
- `htsid`
- `grant_type`

의미:

- `appkey`, `appsecret`, `secretkey`는 앱 키/시크릿
- `cano`는 계좌번호 앞 8자리
- `acnt_prdt_cd`는 계좌번호 뒤 2자리
- `htsid`는 HTS ID
- `grant_type`는 기본적으로 `client_credentials`

즉, help에 보이는 플래그만 채워도 되는 API가 많습니다.

## 설정 값 해석 순서

중요한 값은 아래 우선순위로 해석됩니다.

1. 환경변수
2. config 파일의 복호화된 값
3. 일부 URL/agent는 기본값

대표 환경변수:

- `KIS_ENV`
- `KIS_CONFIG`
- `KIS_REAL_APP_KEY`
- `KIS_REAL_APP_SECRET`
- `KIS_DEMO_APP_KEY`
- `KIS_DEMO_APP_SECRET`
- `KIS_REAL_ACCOUNT_NO`
- `KIS_REAL_ACCOUNT_PRODUCT_CODE`
- `KIS_REAL_HTS_ID`
- `KIS_DEMO_ACCOUNT_NO`
- `KIS_DEMO_ACCOUNT_PRODUCT_CODE`
- `KIS_DEMO_HTS_ID`
- `KIS_USER_AGENT`

config 관련 보조 명령:

- `kis-trading-cli config path`
- `kis-trading-cli config init`
- `kis-trading-cli config set-secret --profile <real|demo> --field <app-key|app-secret|account-no|hts-id> --stdin`
- `kis-trading-cli config seal`
- `kis-trading-cli config key status`
- `kis-trading-cli config key backup [--output <PATH>]`
- `kis-trading-cli config key rotate [--backup <PATH>]`
- `kis-trading-cli config key import --input <PATH>`

키 운영 기본 규칙:

1. 먼저 `config key status`
2. 회전이나 import 전 `config key backup`
3. 현재 config를 유지한 채 key를 교체하려면 `config key rotate`
4. 동일한 config 상태를 복구할 때만 `config key import`

설명:

- `rotate`는 새 active key를 만들고 이전 key는 keyring의 previous key로 유지합니다.
- `import`는 현재 config의 암호문을 실제로 복호화할 수 있는 key만 허용합니다.
- 회전 전 backup key는 회전 전 config snapshot과 짝이 맞습니다.

## 권장 탐색 절차

LLM이 처음 이 CLI를 사용할 때 권장 순서는 아래와 같습니다.

1. `kis-trading-cli catalog summary --compact`
2. 필요한 카테고리 찾기
3. `kis-trading-cli <category> --help`
4. 대상 API 선택
5. `kis-trading-cli <category> <api> --help`
6. 필수 플래그를 채워 실행

## 작업별 명령 매핑

### 설정

- config 파일 경로 확인: `kis-trading-cli config path`
- config 템플릿 생성: `kis-trading-cli config init`
- 암호화된 비밀값 저장: `kis-trading-cli config set-secret`
- 기존 평문 비밀값 암호화: `kis-trading-cli config seal`
- key 상태 확인: `kis-trading-cli config key status`
- key 백업: `kis-trading-cli config key backup`
- key 회전: `kis-trading-cli config key rotate`
- key 복원/이관: `kis-trading-cli config key import`

### 카탈로그 탐색

- 카테고리/개수 요약: `kis-trading-cli catalog summary`
- 전체 manifest JSON 출력: `kis-trading-cli catalog export`

### 인증

- OAuth 접근토큰 발급/재사용: `kis-trading-cli auth token`

### 국내주식

- 현재가 조회: `kis-trading-cli domestic-stock inquire-price`
- 기본 종목정보: `kis-trading-cli domestic-stock search-stock-info`
- 잔고조회: `kis-trading-cli domestic-stock inquire-balance`
- 주문가능조회: `kis-trading-cli domestic-stock inquire-psbl-order`
- 현금주문: `kis-trading-cli domestic-stock order-cash`
- 정정/취소: `kis-trading-cli domestic-stock order-rvsecncl`
- 주문체결조회: `kis-trading-cli domestic-stock inquire-daily-ccld`

### 해외주식

- 카테고리 help: `kis-trading-cli overseas-stock --help`
- 예수금/잔고/주문 관련 API는 이 카테고리 아래에 있습니다.

### 선물옵션

- 국내선물옵션: `kis-trading-cli domestic-futureoption --help`
- 해외선물옵션: `kis-trading-cli overseas-futureoption --help`

## 파라미터 규칙

- CLI 플래그 이름은 원본 파라미터의 `_`를 `-`로 바꾼 형태입니다.
- 예: `FID_INPUT_ISCD`가 아니라 원본 manifest의 파라미터명이 소문자 스네이크였다면
  `--fid-input-iscd`처럼 호출합니다.
- 필수 인자는 `--help`의 Usage 줄에 나타납니다.
- 설명에 `[default: config ...]`가 보이면 config 자동 주입 값입니다.

## 출력 규칙

- 성공/실패 모두 stdout/stderr는 일반 CLI 규칙을 따릅니다.
- 정상 응답 본문은 JSON입니다.
- `--compact`를 붙이면 한 줄 JSON이 됩니다.
- KIS 응답에서 `rt_cd != "0"`이면 실패로 취급해야 합니다.
- 실패 stderr는 JSON envelope로 출력됩니다.

오류 분류 규칙:

- `api_error`
  - KIS가 HTTP 오류를 반환했거나 `rt_cd != "0"`을 반환한 경우
- `program_error`
  - CLI 입력 오류, config/key 문제, 네트워크 실패, 파일 I/O 문제, 내부 처리 실패

LLM이 우선 읽을 필드:

- `error_type`
- `message`
- `llm_hint.summary`
- `llm_hint.retryable`
- `llm_hint.next_action`
- `api_error.msg_cd`
- `api_error.msg1`
- `program_error.category`
- `causes`

## 실행 예시

실전 현재가 조회:

```bash
kis-trading-cli --env real domestic-stock inquire-price \
  --fid-cond-mrkt-div-code J \
  --fid-input-iscd 005930
```

모의 잔고조회:

```bash
kis-trading-cli --env demo domestic-stock inquire-balance \
  --afhr-flpr-yn N \
  --inqr-dvsn 01 \
  --unpr-dvsn 01 \
  --fund-sttl-icld-yn N \
  --fncg-amt-auto-rdpt-yn N \
  --prcs-dvsn 00
```

기계가 읽는 manifest 추출:

```bash
kis-trading-cli catalog export --compact
```

## 피해야 할 가정

- raw URL을 직접 조립해서 호출하는 도구라고 가정하지 않습니다.
- help에 안 보이는 인증/계좌 파라미터를 수동으로 항상 넘길 필요는 없습니다.
- 환경변수까지 암호화해서 넘겨야 한다고 가정하지 않습니다.
- 회전 전 backup key만 있으면 회전 후 current config도 바로 읽을 수 있다고 가정하지 않습니다.
- 모든 API가 실전/모의에서 동일 TR ID를 쓰는 것은 아닙니다.
- 주문 API 일부는 환경/입력에 따라 별도 TR ID resolver를 사용합니다.

## 문서 역할 구분

- `README.md`: 사람용 소개와 빠른 시작
- `docs/LLM_GUIDE.md`: LLM용 호출 규칙과 작업 매핑
- `docs/CLI_REFERENCE.md`: 전체 카테고리/명령 목록
- `data/kis_api_manifest.json`: 가장 엄밀한 기계 판독용 원본
