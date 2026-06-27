# x402 레퍼럴 수수료 + 유료 검증 티어 — 구현 핸드오프

다른 AI/개발자에게 전달하는 명세. 코드는 없음. 무엇을·왜·어디에 만들지만 기술.
대상 레포: OnchainAI (Rust, Leptos SSR + Axum + sqlx + Postgres/Supabase).

---

## 0. 배경 — 지금 시스템이 이미 하는 것 (건드리지 말 것)

- MCP 서버: `POST /mcp`, JSON-RPC 2.0, 읽기 전용 도구 4개 (`search_tools`, `get_tool_detail`, `list_categories`, `get_install_guide`). 위치 `src/server/mcp.rs`.
- 공개 노출 게이트 = `PUBLIC_TOOL_WHERE` (서버 `src/server/mcp.rs` + DB RLS 정책, migrations 011/015). 도구가 공개되려면 전부 만족:
  - `approval_status = 'approved'` (운영자 승인)
  - `relevance_status = 'accepted'` (크립토 관련성)
  - `install_risk_level <> 'critical'`
  - `quarantined_at IS NULL`
  - 백필 노이즈 아님
- x402는 현재 **메타데이터일 뿐**: `tools.pricing = 'x402'` + `tools.x402_price` (텍스트). 결제 실행 코드 없음.
- 도구 소유권 클레임 신청 기능 존재: `request_tool_claim` (migration 007). 단 검증·강제는 아직 약함.

**핵심 원칙: 이 디렉토리는 자금을 보관/이동하지 않는다. 레퍼럴(수익분배)만. 커스터디 모델(facilitator 프록시)은 이번 범위 밖.**

---

## 1. 채택안: A — 레퍼럴 / 수익분배 (커스터디 없음)

### 목표
유저(에이전트)가 네 디렉토리에서 발견한 x402 유료 도구를 호출해 결제할 때, 정산 단계에서 **네 지갑 주소가 수수료 일부를 분배(split)받는다.** 디렉토리는 결제 경로에 끼지 않음 — 돈을 만지지 않음.

### 작동 개념 (말로)
1. 디렉토리는 각 x402 도구에 대해 "이 도구를 우리 통해 호출하면 referrer = 우리 지갑"이라는 메타를 함께 노출.
2. 에이전트가 그 도구를 호출할 때 결제 요청에 referrer 정보가 실림.
3. 도구(또는 그 도구가 쓰는 x402 facilitator)가 정산 시, 합의된 비율만큼 디렉토리 지갑으로 분배.
4. 디렉토리는 클릭/안내 횟수를 자체 기록(어트리뷰션)해서 분배가 맞는지 대조.

### 강제력 한계 (반드시 명시)
- split은 업스트림 도구 또는 facilitator의 협조에 의존. 디렉토리가 강제 못 함.
- 따라서 두 트랙 병행:
  - **트랙 1 (협조형)**: 도구 등록 시 운영자/도구주인이 "referral split N%" 합의. 온체인 split 또는 정기 정산.
  - **트랙 2 (어트리뷰션형)**: split 미지원 도구는 디렉토리가 referral 클릭/전환만 기록 → 나중에 도구 측과 정산 협상 근거. 돈은 자동으로 안 들어옴, 데이터만 확보.

---

## 2. 데이터 모델 변경 (DB) — 새 마이그레이션

새 파일: `migrations/017_x402_referral.sql` (다음 번호). RLS 켜고, 게이트 정책 추가.

### `tools` 테이블에 컬럼 추가
- `referral_enabled BOOLEAN DEFAULT false` — 이 도구에 레퍼럴 적용되는지.
- `referral_bps INTEGER` — 수수료 비율 basis points (예: 250 = 2.5%). NULL이면 미설정.
- `referral_payout_address TEXT` — 디렉토리가 받을 지갑 주소(체인별 필요시 컬럼 분리 or 체인 명시).
- `referral_model TEXT` — `'split'`(트랙1) | `'attribution'`(트랙2).
- `x402_pay_to_address TEXT` — 도구가 결제받는 주소(정직성 검증용, 4번 참조).

### 새 테이블: `referral_events` — 어트리뷰션 로그
- `id`, `tool_id`(FK), `event_type`(`'view'|'install_guide'|'click_out'|'reported_settlement'`),
- `referrer_session`(익명 세션/유저, nullable), `created_at`,
- `amount`(보고된 정산액, nullable), `tx_hash`(온체인 증빙, nullable), `chain`(nullable).
- 목적: 전환 추적 + 정산 대조. 개인정보 최소(세션 해시만, 지갑주소 raw 저장 주의).

### 새 테이블 (선택): `referral_payouts` — 디렉토리가 받은 정산 집계
- `period`, `tool_id`, `total_amount`, `currency`, `tx_hash`, `verified_at`.

---

## 3. 서버 / MCP 노출 변경

### `src/server/mcp.rs`
- `get_tool_detail`, `get_install_guide` 응답에 레퍼럴 메타 포함(노출 OK인 경우만):
  - `referral_enabled`, `referral_bps`, `referral_payout_address`, `referral_model`.
- `get_install_guide`: x402 유료 도구면 안내 텍스트에 한 줄 추가 — "이 도구는 호출 시 x402 결제(USDC 등). 에이전트에 지갑 연결 필요. 본 디렉토리 경유 호출 시 referral 적용."
- **민감정보 주의**: payout_address는 디렉토리 소유 공개주소라 노출 가능. 단 도구주인 개인 정산 데이터는 노출 금지. 기존 `sanitize_tool_for_public_response`(src/models/tool.rs) 패턴 따를 것.

### 어트리뷰션 기록
- `get_install_guide` 또는 별도 "click-out" 서버 함수 호출 시 `referral_events`에 insert.
- 레이트 리밋 적용(기존 `src/server/rate_limit.rs` 패턴) — 어트리뷰션 어뷰징 방지.

---

## 4. 유료 도구 검증 강화 (필수 — 돈 받으면 책임 생김)

무료 도구 오류 = 짜증. **유료 x402 + 수수료 도구가 사기면 디렉토리가 공범 + 수수료 챙긴 꼴 = 법적 노출.** 그래서 레퍼럴 켜진 도구는 게이트 한 단 더 통과해야 MCP 노출.

### `tools`에 검증 플래그 추가 (위 마이그레이션에 포함)
- `payment_verified BOOLEAN DEFAULT false` — 결제 정산 주소가 도구 주인 것으로 확인됨.
- `x402_endpoint_verified BOOLEAN DEFAULT false` — 엔드포인트가 실제 402 응답 + 결제 핸드셰이크 정상.
- `price_verified BOOLEAN DEFAULT false` — 광고가(x402_price) = 실제 청구가 일치.

### 검증 항목 (운영자/자동 잡)
| 항목 | 방법 |
|------|------|
| 소유권 | 기존 `request_tool_claim`(007) 강제화 — 결제받는 주소가 클레임한 주인 것임을 서명/온체인으로 증명 |
| 엔드포인트 라이브니스 | 헬스체크 잡: x402 endpoint에 호출 → 402 + 올바른 결제 헤더 오는지 |
| 가격 정직성 | 402 응답의 청구가 vs `x402_price` 비교 |
| 정산 주소 | `x402_pay_to_address` 온체인 존재/형식 확인 |

### 강화 게이트
- `PUBLIC_TOOL_WHERE`를 확장하거나 별도 `MONETIZED_TOOL_WHERE` 추가:
  - `referral_enabled = false` 도구는 기존 게이트만.
  - `referral_enabled = true` 도구는 기존 게이트 **+** `payment_verified AND x402_endpoint_verified AND price_verified` 까지 만족해야 MCP/공개 노출.
- 서버(`mcp.rs`)와 DB RLS 정책 **양쪽** 동일하게 반영(현재 011/015처럼 이중 게이트 유지 — 우회 방지).

---

## 5. 운영자 어드민 UI (노코드 운영용)

`src/pages/admin/` 에 추가:
- 도구 심사(`tools.rs`) 또는 신규 섹션에서 도구별 레퍼럴 설정: enabled 토글, bps 입력, payout 주소, model 선택.
- 검증 플래그 상태 표시(payment/endpoint/price verified 뱃지) + 수동 재검증 버튼.
- 레퍼럴 대시보드: `referral_events` 집계, 전환수, 보고된 정산액.
- 사이트 설정(`settings.rs`)에 디렉토리 기본 payout 주소 + 기본 bps 추가 (개별 도구가 오버라이드).

---

## 6. 공개 UX (유저 이해/사용 쉽게)

- 도구 상세(`src/components/tool_detail_content.rs`): x402 뱃지 옆에 "유료 · 호출 시 USDC 결제 · 지갑 필요" 한 줄 + AgentKit 등 지갑 연결 가이드 링크.
- 홈/About에 "Connect MCP" 카드: 엔드포인트 URL `https://www.onchain-ai.xyz/mcp` + Claude/Cursor mcpServers JSON 복사 버튼.
- "AI한테 'onchain-ai에서 크립토 도구 찾아줘'라고만 말하세요" 5초 카피.
- referral은 유저에게 **투명 고지**(수수료 받는다는 사실) — 신뢰 + 규제상 안전.

---

## 7. 범위 밖 (이번에 하지 말 것)

- Facilitator/프록시 게이트웨이(B 모델): 디렉토리가 결제 경로에 끼어 자금 보관/정산. 커스터디·자금이체 규제·보안 감사 = 별도 대형 프로젝트. **지금 안 함.**
- 디렉토리 지갑이 유저 자금을 한시라도 보관하는 모든 설계 금지.

---

## 8. 구현 순서 (다른 AI에게 지시할 단계)

1. 마이그레이션 017: 컬럼·테이블·강화 게이트(서버+RLS).
2. `mcp.rs`: 레퍼럴 메타 노출 + x402 안내 텍스트 + 어트리뷰션 기록.
3. 검증 잡: 엔드포인트 라이브니스·가격·소유권 체크 → 플래그 갱신.
4. 어드민 UI: 도구별 레퍼럴 설정 + 검증 상태 + 대시보드.
5. 공개 UX: 상세 페이지 안내 + Connect MCP 카드.
6. 테스트: 게이트(검증 안 된 유료도구 비노출), 어트리뷰션 기록, sanitize(민감정보 비노출). 기존 `cargo test --features ssr` + clippy/fmt 통과.

## 9. 검증 주의 (실측 필요)

- x402 spec / facilitator API / split·referrer 필드는 빠르게 진화 중. **구현 전 현재 x402 공식 스펙 실측 확인.** 내 명세의 "referrer/split" 메커니즘은 개념이며, 실제 필드명·정산 방식은 최신 x402 문서 기준으로 맞출 것.
- USDC/EIP-3009/체인별 정산 주소 형식 확인.

---

### 한 줄 요약
A(레퍼럴, 커스터디 없음) 채택. 도구에 referral 메타+검증 플래그 추가 → 유료 도구는 강화 게이트 통과해야 MCP 노출 → 어트리뷰션 로그로 정산 대조 → 운영자 UI/공개 UX/투명고지. facilitator 커스터디는 범위 밖.
