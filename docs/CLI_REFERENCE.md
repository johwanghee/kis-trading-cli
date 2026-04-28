# CLI Reference

> Generated from `data/kis_api_manifest.json`. Edit the manifest or generator, not this file.

- Source repo: `https://github.com/koreainvestment/open-trading-api`
- Source commit: `33e0e1e65cd1c8c8b639531483ec0b327087bab1`
- Categories: `8`
- APIs: `166`

## Top-level commands

- `config`: local config file path/template management
- `catalog`: embedded manifest summary/export
- `auth`: 한국투자증권의 auth OPEN API를 활용합니다. (2 APIs)
- `domestic-bond`: 한국투자증권의 장내채권 OPEN API를 활용합니다. (14 APIs)
- `domestic-futureoption`: 한국투자증권의 국내선물옵션 OPEN API를 활용합니다. (20 APIs)
- `domestic-stock`: 한국투자증권의 국내주식 OPEN API를 활용합니다. (74 APIs)
- `elw`: 한국투자증권의 ELW OPEN API를 활용합니다. (1 APIs)
- `etfetn`: 한국투자증권의 ETF/ETN OPEN API를 활용합니다. (2 APIs)
- `overseas-futureoption`: 한국투자증권의 해외선물옵션 OPEN API를 활용합니다. (19 APIs)
- `overseas-stock`: 한국투자증권의 해외주식 OPEN API를 활용합니다. (34 APIs)

## Global options

- `--env <demo|real>`
- `--config <PATH>`
- `--compact`

## `auth`

- Description: 한국투자증권의 auth OPEN API를 활용합니다.
- Config source file: `auth.json`
- API count: `2`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `token` | 접근토큰발급(P) | `POST` | `/oauth2/tokenP` | 1 |
| `ws-token` | 실시간 (웹소켓) 접속키 발급 | `POST` | `/oauth2/Approval` | 1 |

## `domestic-bond`

- Description: 한국투자증권의 장내채권 OPEN API를 활용합니다.
- Config source file: `domestic_bond.json`
- API count: `14`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `avg-unit` | 장내채권 평균단가조회 | `GET` | `/uapi/domestic-bond/v1/quotations/avg-unit` | 5 |
| `buy` | 장내채권 매수주문 | `POST` | `/uapi/domestic-bond/v1/trading/buy` | 9 |
| `inquire-asking-price` | 장내채권현재가(호가) | `GET` | `/uapi/domestic-bond/v1/quotations/inquire-asking-price` | 2 |
| `inquire-balance` | 장내채권 잔고조회 | `GET` | `/uapi/domestic-bond/v1/trading/inquire-balance` | 3 |
| `inquire-ccnl` | 장내채권현재가(체결) | `GET` | `/uapi/domestic-bond/v1/quotations/inquire-ccnl` | 2 |
| `inquire-daily-ccld` | 장내채권 주문체결내역 | `GET` | `/uapi/domestic-bond/v1/trading/inquire-daily-ccld` | 8 |
| `inquire-daily-price` | 장내채권현재가(일별) | `GET` | `/uapi/domestic-bond/v1/quotations/inquire-daily-price` | 2 |
| `inquire-price` | 장내채권현재가(시세) | `GET` | `/uapi/domestic-bond/v1/quotations/inquire-price` | 2 |
| `inquire-psbl-order` | 장내채권 매수가능조회 | `GET` | `/uapi/domestic-bond/v1/trading/inquire-psbl-order` | 2 |
| `inquire-psbl-rvsecncl` | 채권정정취소가능주문조회 | `GET` | `/uapi/domestic-bond/v1/trading/inquire-psbl-rvsecncl` | 4 |
| `issue-info` | 장내채권 발행정보 | `GET` | `/uapi/domestic-bond/v1/quotations/issue-info` | 2 |
| `order-rvsecncl` | 장내채권 정정취소주문 | `POST` | `/uapi/domestic-bond/v1/trading/order-rvsecncl` | 9 |
| `search-bond-info` | 장내채권 기본조회 | `GET` | `/uapi/domestic-bond/v1/quotations/search-bond-info` | 2 |
| `sell` | 장내채권 매도주문 | `POST` | `/uapi/domestic-bond/v1/trading/sell` | 13 |

## `domestic-futureoption`

- Description: 한국투자증권의 국내선물옵션 OPEN API를 활용합니다.
- Config source file: `domestic_futureoption.json`
- API count: `20`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `display-board-top` | 국내선물 기초자산 시세 | `GET` | `/uapi/domestic-futureoption/v1/quotations/display-board-top` | 6 |
| `exp-price-trend` | 선물옵션 일중예상체결추이 | `GET` | `/uapi/domestic-futureoption/v1/quotations/exp-price-trend` | 2 |
| `inquire-asking-price` | 선물옵션 시세호가 | `GET` | `/uapi/domestic-futureoption/v1/quotations/inquire-asking-price` | 2 |
| `inquire-balance` | 선물옵션 잔고현황 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-balance` | 2 |
| `inquire-balance-settlement-pl` | 선물옵션 잔고정산손익내역 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-balance-settlement-pl` | 1 |
| `inquire-balance-valuation-pl` | 선물옵션 잔고평가손익내역 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-balance-valuation-pl` | 2 |
| `inquire-ccnl` | 선물옵션 주문체결내역조회 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-ccnl` | 8 |
| `inquire-ccnl-bstime` | 선물옵션 기준일체결내역 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-ccnl-bstime` | 3 |
| `inquire-daily-amount-fee` | 선물옵션기간약정수수료일별 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-daily-amount-fee` | 2 |
| `inquire-daily-fuopchartprice` | 선물옵션기간별시세(일/주/월/년) | `GET` | `/uapi/domestic-futureoption/v1/quotations/inquire-daily-fuopchartprice` | 5 |
| `inquire-deposit` | 선물옵션 총자산현황 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-deposit` | 0 |
| `inquire-ngt-balance` | (야간)선물옵션 잔고현황 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-ngt-balance` | 3 |
| `inquire-ngt-ccnl` | (야간)선물옵션 주문체결 내역조회 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-ngt-ccnl` | 10 |
| `inquire-price` | 선물옵션 시세 | `GET` | `/uapi/domestic-futureoption/v1/quotations/inquire-price` | 2 |
| `inquire-psbl-ngt-order` | (야간)선물옵션 주문가능 조회 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-psbl-ngt-order` | 5 |
| `inquire-psbl-order` | 선물옵션 주문가능 | `GET` | `/uapi/domestic-futureoption/v1/trading/inquire-psbl-order` | 4 |
| `inquire-time-fuopchartprice` | 선물옵션 분봉조회 | `GET` | `/uapi/domestic-futureoption/v1/quotations/inquire-time-fuopchartprice` | 7 |
| `ngt-margin-detail` | (야간)선물옵션 증거금 상세 | `GET` | `/uapi/domestic-futureoption/v1/trading/ngt-margin-detail` | 1 |
| `order` | 선물옵션 주문 | `POST` | `/uapi/domestic-futureoption/v1/trading/order` | 11 |
| `order-rvsecncl` | 선물옵션 정정취소주문 | `POST` | `/uapi/domestic-futureoption/v1/trading/order-rvsecncl` | 11 |

## `domestic-stock`

- Description: 한국투자증권의 국내주식 OPEN API를 활용합니다.
- Config source file: `domestic_stock.json`
- API count: `74`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `chk-holiday` | 국내휴장일조회 | `GET` | `/uapi/domestic-stock/v1/quotations/chk-holiday` | 1 |
| `comp-program-trade-daily` | 프로그램매매 종합현황(일별) | `GET` | `/uapi/domestic-stock/v1/quotations/comp-program-trade-daily` | 4 |
| `daily-loan-trans` | 종목별 일별 대차거래추이 | `GET` | `/uapi/domestic-stock/v1/quotations/daily-loan-trans` | 4 |
| `daily-short-sale` | 국내주식 공매도 일별추이 | `GET` | `/uapi/domestic-stock/v1/quotations/daily-short-sale` | 4 |
| `estimate-perform` | 국내주식 종목추정실적 | `GET` | `/uapi/domestic-stock/v1/quotations/estimate-perform` | 1 |
| `fluctuation` | 국내주식 등락률 순위 | `GET` | `/uapi/domestic-stock/v1/ranking/fluctuation` | 14 |
| `foreign-institution-total` | 국내기관_외국인 매매종목가집계 | `GET` | `/uapi/domestic-stock/v1/quotations/foreign-institution-total` | 6 |
| `frgnmem-pchs-trend` | 종목별 외국계 순매수추이 | `GET` | `/uapi/domestic-stock/v1/quotations/frgnmem-pchs-trend` | 3 |
| `frgnmem-trade-trend` | 회원사 실시간 매매동향(틱) | `GET` | `/uapi/domestic-stock/v1/quotations/frgnmem-trade-trend` | 6 |
| `inquire-account-balance` | 투자계좌자산현황조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-account-balance` | 2 |
| `inquire-asking-price-exp-ccn` | 주식현재가 호가/예상체결 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-asking-price-exp-ccn` | 2 |
| `inquire-balance` | 주식잔고조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-balance` | 6 |
| `inquire-balance-rlz-pl` | 주식잔고조회_실현손익 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-balance-rlz-pl` | 8 |
| `inquire-ccnl` | 주식현재가 체결 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-ccnl` | 2 |
| `inquire-credit-psamount` | 신용매수가능조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-credit-psamount` | 6 |
| `inquire-daily-ccld` | 주식일별주문체결조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-daily-ccld` | 11 |
| `inquire-daily-indexchartprice` | 국내주식업종기간별시세(일/주/월/년) | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-daily-indexchartprice` | 5 |
| `inquire-daily-itemchartprice` | 국내주식기간별시세(일/주/월/년) | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-daily-itemchartprice` | 6 |
| `inquire-daily-overtimeprice` | 주식현재가 시간외일자별주가 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-daily-overtimeprice` | 2 |
| `inquire-daily-price` | 주식현재가 일자별 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-daily-price` | 4 |
| `inquire-daily-trade-volume` | 종목별일별매수매도체결량 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-daily-trade-volume` | 5 |
| `inquire-elw-price` | ELW 현재가 시세 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-elw-price` | 2 |
| `inquire-index-daily-price` | 국내업종 일자별지수 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-index-daily-price` | 4 |
| `inquire-index-price` | 국내업종 현재지수 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-index-price` | 2 |
| `inquire-investor` | 주식현재가 투자자 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-investor` | 2 |
| `inquire-investor-daily-by-market` | 시장별 투자자매매동향(일별) | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-investor-daily-by-market` | 6 |
| `inquire-investor-time-by-market` | 시장별 투자자매매동향(시세) | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-investor-time-by-market` | 2 |
| `inquire-member` | 주식현재가 회원사 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-member` | 2 |
| `inquire-member-daily` | 주식현재가 회원사 종목매매동향 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-member-daily` | 6 |
| `inquire-overtime-asking-price` | 국내주식 시간외호가 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-overtime-asking-price` | 2 |
| `inquire-overtime-price` | 국내주식 시간외현재가 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-overtime-price` | 2 |
| `inquire-period-profit` | 기간별손익일별합산조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-period-profit` | 6 |
| `inquire-period-trade-profit` | 기간별매매손익현황조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-period-trade-profit` | 5 |
| `inquire-price` | 주식현재가 시세 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-price` | 2 |
| `inquire-price-2` | 주식현재가 시세2 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-price-2` | 2 |
| `inquire-psbl-order` | 매수가능조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-psbl-order` | 5 |
| `inquire-psbl-rvsecncl` | 주식정정취소가능주문조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-psbl-rvsecncl` | 2 |
| `inquire-psbl-sell` | 매도가능수량조회 | `GET` | `/uapi/domestic-stock/v1/trading/inquire-psbl-sell` | 1 |
| `inquire-time-dailychartprice` | 주식일별분봉조회 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-time-dailychartprice` | 6 |
| `inquire-time-indexchartprice` | 업종 분봉조회 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-time-indexchartprice` | 5 |
| `inquire-time-itemchartprice` | 주식당일분봉조회 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-time-itemchartprice` | 5 |
| `inquire-time-itemconclusion` | 주식현재가 당일시간대별체결 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-time-itemconclusion` | 3 |
| `inquire-time-overtimeconclusion` | 주식현재가 시간외시간별체결 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-time-overtimeconclusion` | 3 |
| `inquire-vi-status` | 변동성완화장치(VI) 현황 | `GET` | `/uapi/domestic-stock/v1/quotations/inquire-vi-status` | 8 |
| `intgr-margin` | 주식통합증거금 현황 | `GET` | `/uapi/domestic-stock/v1/trading/intgr-margin` | 3 |
| `intstock-multprice` | 관심종목(멀티종목) 시세조회 | `GET` | `/uapi/domestic-stock/v1/quotations/intstock-multprice` | 2 |
| `intstock-stocklist-by-group` | 관심종목 그룹별 종목조회 | `GET` | `/uapi/domestic-stock/v1/quotations/intstock-stocklist-by-group` | 8 |
| `invest-opbysec` | 국내주식 증권사별 투자의견 | `GET` | `/uapi/domestic-stock/v1/quotations/invest-opbysec` | 6 |
| `invest-opinion` | 국내주식 종목투자의견 | `GET` | `/uapi/domestic-stock/v1/quotations/invest-opinion` | 5 |
| `investor-program-trade-today` | 프로그램매매 투자자매매동향(당일) | `GET` | `/uapi/domestic-stock/v1/quotations/investor-program-trade-today` | 1 |
| `investor-trade-by-stock-daily` | 종목별 투자자매매동향(일별) | `GET` | `/uapi/domestic-stock/v1/quotations/investor-trade-by-stock-daily` | 5 |
| `investor-trend-estimate` | 종목별 외인기관 추정가집계 | `GET` | `/uapi/domestic-stock/v1/quotations/investor-trend-estimate` | 1 |
| `market-cap` | 국내주식 시가총액 상위 | `GET` | `/uapi/domestic-stock/v1/ranking/market-cap` | 9 |
| `news-title` | 종합 시황/공시(제목) | `GET` | `/uapi/domestic-stock/v1/quotations/news-title` | 8 |
| `order-cash` | 주식주문(현금) | `POST` | `/uapi/domestic-stock/v1/trading/order-cash` | 8 |
| `order-credit` | 주식주문(신용) | `POST` | `/uapi/domestic-stock/v1/trading/order-credit` | 23 |
| `order-resv` | 주식예약주문 | `POST` | `/uapi/domestic-stock/v1/trading/order-resv` | 6 |
| `order-resv-ccnl` | 주식예약주문조회 | `GET` | `/uapi/domestic-stock/v1/trading/order-resv-ccnl` | 8 |
| `order-resv-rvsecncl` | 주식예약주문정정취소 | `POST` | `/uapi/domestic-stock/v1/trading/order-resv-rvsecncl` | 4 |
| `order-rvsecncl` | 주식주문(정정취소) | `POST` | `/uapi/domestic-stock/v1/trading/order-rvsecncl` | 8 |
| `pension-inquire-balance` | 퇴직연금 잔고조회 | `GET` | `/uapi/domestic-stock/v1/trading/pension/inquire-balance` | 2 |
| `pension-inquire-daily-ccld` | 퇴직연금 미체결내역 | `GET` | `/uapi/domestic-stock/v1/trading/pension/inquire-daily-ccld` | 4 |
| `pension-inquire-deposit` | 퇴직연금 예수금조회 | `GET` | `/uapi/domestic-stock/v1/trading/pension/inquire-deposit` | 1 |
| `pension-inquire-present-balance` | 퇴직연금 체결기준잔고 | `GET` | `/uapi/domestic-stock/v1/trading/pension/inquire-present-balance` | 1 |
| `pension-inquire-psbl-order` | 퇴직연금 매수가능조회 | `GET` | `/uapi/domestic-stock/v1/trading/pension/inquire-psbl-order` | 5 |
| `period-rights` | 기간별계좌권리현황조회 | `GET` | `/uapi/domestic-stock/v1/trading/period-rights` | 8 |
| `program-trade-by-stock` | 종목별 프로그램매매추이(체결) | `GET` | `/uapi/domestic-stock/v1/quotations/program-trade-by-stock` | 2 |
| `program-trade-by-stock-daily` | 종목별 프로그램매매추이(일별) | `GET` | `/uapi/domestic-stock/v1/quotations/program-trade-by-stock-daily` | 3 |
| `psearch-result` | 종목조건검색조회 | `GET` | `/uapi/domestic-stock/v1/quotations/psearch-result` | 1 |
| `psearch-title` | 종목조건검색 목록조회 | `GET` | `/uapi/domestic-stock/v1/quotations/psearch-title` | 1 |
| `search-info` | 상품기본조회 | `GET` | `/uapi/domestic-stock/v1/quotations/search-info` | 2 |
| `search-stock-info` | 주식기본조회 | `GET` | `/uapi/domestic-stock/v1/quotations/search-stock-info` | 2 |
| `volume-power` | 국내주식 체결강도 상위 | `GET` | `/uapi/domestic-stock/v1/ranking/volume-power` | 9 |
| `volume-rank` | 거래량순위 | `GET` | `/uapi/domestic-stock/v1/quotations/volume-rank` | 11 |

## `elw`

- Description: 한국투자증권의 ELW OPEN API를 활용합니다.
- Config source file: `elw.json`
- API count: `1`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `volume-rank` | ELW 거래량순위 | `GET` | `/uapi/elw/v1/ranking/volume-rank` | 15 |

## `etfetn`

- Description: 한국투자증권의 ETF/ETN OPEN API를 활용합니다.
- Config source file: `etfetn.json`
- API count: `2`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `inquire-price` | ETF/ETN 현재가 | `GET` | `/uapi/etfetn/v1/quotations/inquire-price` | 2 |
| `nav-comparison-trend` | NAV 비교추이(종목) | `GET` | `/uapi/etfetn/v1/quotations/nav-comparison-trend` | 2 |

## `overseas-futureoption`

- Description: 한국투자증권의 해외선물옵션 OPEN API를 활용합니다.
- Config source file: `overseas_futureoption.json`
- API count: `19`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `daily-ccnl` | 해외선물 체결추이(일간) | `GET` | `/uapi/overseas-futureoption/v1/quotations/daily-ccnl` | 8 |
| `inquire-asking-price` | 해외선물 호가 | `GET` | `/uapi/overseas-futureoption/v1/quotations/inquire-asking-price` | 1 |
| `inquire-ccld` | 해외선물옵션 당일주문내역조회 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-ccld` | 5 |
| `inquire-daily-ccld` | 해외선물옵션 일별 체결내역 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-daily-ccld` | 9 |
| `inquire-daily-order` | 해외선물옵션 일별 주문내역 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-daily-order` | 8 |
| `inquire-deposit` | 해외선물옵션 예수금현황 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-deposit` | 2 |
| `inquire-period-ccld` | 해외선물옵션 기간계좌손익 일별 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-period-ccld` | 7 |
| `inquire-period-trans` | 해외선물옵션 기간계좌거래내역 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-period-trans` | 7 |
| `inquire-price` | 해외선물종목현재가 | `GET` | `/uapi/overseas-futureoption/v1/quotations/inquire-price` | 1 |
| `inquire-psamount` | 해외선물옵션 주문가능조회 | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-psamount` | 4 |
| `inquire-time-futurechartprice` | 해외선물 분봉조회 | `GET` | `/uapi/overseas-futureoption/v1/quotations/inquire-time-futurechartprice` | 8 |
| `inquire-unpd` | 해외선물옵션 미결제내역조회(잔고) | `GET` | `/uapi/overseas-futureoption/v1/trading/inquire-unpd` | 3 |
| `margin-detail` | 해외선물옵션 증거금상세 | `GET` | `/uapi/overseas-futureoption/v1/trading/margin-detail` | 2 |
| `opt-asking-price` | 해외옵션 호가 | `GET` | `/uapi/overseas-futureoption/v1/quotations/opt-asking-price` | 1 |
| `opt-price` | 해외옵션종목현재가 | `GET` | `/uapi/overseas-futureoption/v1/quotations/opt-price` | 1 |
| `order` | 해외선물옵션 주문 | `POST` | `/uapi/overseas-futureoption/v1/trading/order` | 14 |
| `order-rvsecncl` | 해외선물옵션 정정취소주문 | `POST` | `/uapi/overseas-futureoption/v1/trading/order-rvsecncl` | 9 |
| `search-contract-detail` | 해외선물 상품기본정보 | `GET` | `/uapi/overseas-futureoption/v1/quotations/search-contract-detail` | 1 |
| `search-opt-detail` | 해외옵션 상품기본정보 | `GET` | `/uapi/overseas-futureoption/v1/quotations/search-opt-detail` | 2 |

## `overseas-stock`

- Description: 한국투자증권의 해외주식 OPEN API를 활용합니다.
- Config source file: `overseas_stock.json`
- API count: `34`

| Command | 설명 | Method | Path | Required flags |
| --- | --- | --- | --- | ---: |
| `algo-ordno` | 해외주식 지정가주문번호조회 | `GET` | `/uapi/overseas-stock/v1/trading/algo-ordno` | 1 |
| `dailyprice` | 해외주식 기간별시세 | `GET` | `/uapi/overseas-price/v1/quotations/dailyprice` | 6 |
| `daytime-order` | 해외주식 미국주간주문 | `POST` | `/uapi/overseas-stock/v1/trading/daytime-order` | 9 |
| `daytime-order-rvsecncl` | 해외주식 미국주간정정취소 | `POST` | `/uapi/overseas-stock/v1/trading/daytime-order-rvsecncl` | 9 |
| `foreign-margin` | 해외증거금 통화별조회 | `GET` | `/uapi/overseas-stock/v1/trading/foreign-margin` | 0 |
| `industry-theme` | 해외주식 업종별시세 | `GET` | `/uapi/overseas-price/v1/quotations/industry-theme` | 5 |
| `inquire-algo-ccnl` | 해외주식 지정가체결내역조회 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-algo-ccnl` | 4 |
| `inquire-asking-price` | 해외주식 현재가 1호가 | `GET` | `/uapi/overseas-price/v1/quotations/inquire-asking-price` | 3 |
| `inquire-balance` | 해외주식 잔고 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-balance` | 2 |
| `inquire-ccnl` | 해외주식 주문체결내역 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-ccnl` | 10 |
| `inquire-daily-chartprice` | 해외주식 종목/지수/환율기간별시세(일/주/월/년) | `GET` | `/uapi/overseas-price/v1/quotations/inquire-daily-chartprice` | 5 |
| `inquire-nccs` | 해외주식 미체결내역 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-nccs` | 2 |
| `inquire-paymt-stdr-balance` | 해외주식 결제기준잔고 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-paymt-stdr-balance` | 3 |
| `inquire-period-profit` | 해외주식 기간손익 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-period-profit` | 7 |
| `inquire-period-trans` | 해외주식 일별거래내역 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-period-trans` | 6 |
| `inquire-present-balance` | 해외주식 체결기준현재잔고 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-present-balance` | 4 |
| `inquire-psamount` | 해외주식 매수가능금액조회 | `GET` | `/uapi/overseas-stock/v1/trading/inquire-psamount` | 3 |
| `inquire-search` | 해외주식조건검색 | `GET` | `/uapi/overseas-price/v1/quotations/inquire-search` | 27 |
| `inquire-time-indexchartprice` | 해외지수분봉조회 | `GET` | `/uapi/overseas-price/v1/quotations/inquire-time-indexchartprice` | 4 |
| `inquire-time-itemchartprice` | 해외주식분봉조회 | `GET` | `/uapi/overseas-price/v1/quotations/inquire-time-itemchartprice` | 9 |
| `order` | 해외주식 주문 | `POST` | `/uapi/overseas-stock/v1/trading/order` | 9 |
| `order-resv` | 해외주식 예약주문접수 | `POST` | `/uapi/overseas-stock/v1/trading/order-resv` | 5 |
| `order-resv-ccnl` | 해외주식 예약주문접수취소 | `POST` | `/uapi/overseas-stock/v1/trading/order-resv-ccnl` | 3 |
| `order-resv-list` | 해외주식 예약주문조회 | `GET` | `/uapi/overseas-stock/v1/trading/order-resv-list` | 6 |
| `order-rvsecncl` | 해외주식 정정취소주문 | `POST` | `/uapi/overseas-stock/v1/trading/order-rvsecncl` | 8 |
| `period-rights` | 해외주식 기간별권리조회 | `GET` | `/uapi/overseas-price/v1/quotations/period-rights` | 6 |
| `price` | 해외주식 현재체결가 | `GET` | `/uapi/overseas-price/v1/quotations/price` | 3 |
| `price-detail` | 해외주식 현재가상세 | `GET` | `/uapi/overseas-price/v1/quotations/price-detail` | 3 |
| `price-fluct` | 해외주식 가격급등락 | `GET` | `/uapi/overseas-stock/v1/ranking/price-fluct` | 6 |
| `quot-inquire-ccnl` | 해외주식 체결추이 | `GET` | `/uapi/overseas-price/v1/quotations/inquire-ccnl` | 5 |
| `rights-by-ice` | 해외주식 권리종합 | `GET` | `/uapi/overseas-price/v1/quotations/rights-by-ice` | 4 |
| `search-info` | 해외주식 상품기본정보 | `GET` | `/uapi/overseas-price/v1/quotations/search-info` | 2 |
| `trade-vol` | 해외주식 거래량순위 | `GET` | `/uapi/overseas-stock/v1/ranking/trade-vol` | 7 |
| `updown-rate` | 해외주식 상승율/하락율 | `GET` | `/uapi/overseas-stock/v1/ranking/updown-rate` | 6 |

