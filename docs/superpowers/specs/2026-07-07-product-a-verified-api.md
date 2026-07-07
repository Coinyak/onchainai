# Product A — "검증된 결정 API" 설계 (Wave 4+ 로드맵)

> Related: [[2026-07-07-okx-x402-infra-waves]] | [[2026-07-07-s-group-strategy-memo]] | [[../../X402_OPEN_LISTING_SPEC]] | [[2026-07-04-free-tier-guardian-spec]] | [[../../../AGENTS.md]]
>
> Date: 2026-07-07
> Status: Roadmap memo — **인프라 Wave 0~3(별도) 완료 후 착수**. K2(`check_endpoint_health`) prod 정산 + L4 probe history 적재가 선행.
> Scope: `check_endpoint_health`의 상위 상품 — 에이전트에게 후보가 아니라 **검증된 단일 정답**을 반환하는 per-call 유료 MCP 툴. 신규 인프라 거의 없음(기존 `probe_x402_endpoint` 백본 재사용).

**본 문서는 구현 코드를 포함하지 않는다.** 검증 범위·흐름·수용 기준만 정의.

---

## A.1 검증 범위 (정직 분리)

**검증 가능(강함·증명)**
- **Liveness**: on-demand 402 핸드셰이크(`probe_x402_endpoint`, 5s·SSRF 가드·64KB 캡). 호출 시점 실측.
- **요금 일치**: 402 PaymentRequirements(실제 청구) vs `tools.x402_price`(광고) 비교. `x402_probe_history` 기록.

**검증 불가(soft·고지 필수)**
- **총비용 최저 아님**: x402 프로브는 프로토콜 수수료만. 실행가/슬리피지/가스 미검증 → "가장 싼" = "x402 수수료 기준"으로 한정 표기.
- **안전 보증 아님**: trust tier·install_risk·리뷰는 큐레이션. 신규 scam 신고 전 통과 가능.
- **태스크 정합성**: function/chain 태그(크롤러 정규화) 의존. 실제 실행 검증 안 함.

## A.2 흐름

1. 태스크 → 무료 search 후보 추출(function/chain 필터).
2. 랭킹: (trust tier → x402 fee)순. 상위 N(예: 3) 선정.
3. on-demand 402 프로브 상위 N 병렬 → liveness + 실제 요금.
4. price-match(광고 vs 실제) 확인.
5. LIVE + price-match 통과 최상위 1개 반환. **아무것도 살아있지 않으면 "검증된 live 도구 없음" 반환(속이지 않음).**
6. 고지: "시각 T 기준 liveness + x402 요금 검증, trust tier X. 실행가/슬리피지 미검증."
7. 캐싱: 동일 태스크 프로브 결과 ~60s 캐시 → 반복 비용 절감, staleness 명시. **캐시 투명성**: 캐시 hit 시 "cached at T" 표기(fresh on-demand 아님 명시).

## A.3 explain_rejection (탈락 가시성, 추가)

결정 1개만 주면 "왜 이거냐/다른 건 왜 탈락"이 안 보여 신뢰 약함. 반환에 **탈락 후보 요약** 포함:
- 각 탈락 후보별 사유: `DEAD`(402 미응답)·`PRICE_MISMATCH`(광고≠실제, ±허용치 초과)·`STALE`(trust tier 낮음)·`NOT_RELEVANT`.
- 이로써 "검증된 1개 + 왜 다른 건 탈락"이 한 응답에 → MVP 신뢰 확보.

## A.4 가격/포지션

- $0.001~0.01/call — `check_endpoint_health`(단발 보험)의 상위. "대신 프로브 돌려 검증된 한 개 고름"이 가치.
- 상품 카피: **"검증된 live + 요금 정직 + 최고 신뢰등급"** (≠ "가장 싼/안전 보증").
- OKX A2MCP 등재 시 infra-waves §3.6 3-step SOP와 묶어 "에이전트 운영 레시피"로 포지션.

## A.5 수용 기준

- [ ] 반환 툴 인용 전부 카탈로그 실재 slug(할루 0).
- [ ] LIVE 0건 시 "검증된 live 도구 없음" 반환(가짜 정답 금지).
- [ ] explain_rejection: 탈락 후보별 사유(DEAD/PRICE_MISMATCH/STALE/NOT_RELEVANT) 포함.
- [ ] 캐시 hit 시 "cached at T" 명시.
- [ ] "시각 T 기준" + "실행가/슬리피지 미검증" 고지 포함.
