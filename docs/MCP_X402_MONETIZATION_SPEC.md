# MCP x402 수익화 스펙 (Axis B · 셀프서비스)

> Related: [[X402_REFERRAL_SPEC]] | [[PRODUCT_ENHANCEMENT_SPEC]] §K2 | [[MCP_AGENT_WORKFLOW]] | [[SECURITY]] | [[../../../AGENTS.md]]
>
> Date: 2026-07-04  
> Status: **Superseded** — `compare_tools`/`export_toolkit` 유료안 폐기. 정본: [[X402_MONETIZATION_SPEC]] (Agent Trust만 유료)  
> Scope: (레거시) Axis B MCP 402 게이트 구현 참고. 신규 과금 SKU 추가 금지.

---

## 1. 목표

1. **축 B (개인/운영자 제작자)**: OnchainAI가 *자기* MCP 기능을 x402로 판매한다. payee는 운영자 지갑, 제3자 자금을 보관·중계하지 않는다.
2. **셀프서비스**: Admin `/admin/settings`에서 premium on/off, pay-to, price, network를 코드 배포 없이 설정한다.
3. **MCP 전송 계층 투명성**: 유료 툴 호출 시 **HTTP 402** + `PAYMENT-REQUIRED` 헤더. JSON-RPC 200 안에 숨기지 않는다.
4. **발견 무료 유지**: `search_tools`, `get_tool_detail`, `list_categories`, `get_dashboard_snapshot`, `get_install_guide`는 항상 무료.

---

## 2. 아키텍처

| 계층 | 역할 |
|------|------|
| DB | `site_settings.mcp_premium_*` (`031_mcp_x402_monetization.sql`) |
| API | `POST /mcp` — premium `tools/call` 시 402 또는 `PAYMENT-SIGNATURE` 검증 후 실행 |
| Admin | `/admin/settings` — MCP premium 필드 (operator self-service) |
| Env | `X402_FACILITATOR_URL` — **결제 검증만** (선택). `ONCHAINAI_MCP_X402_DEV_ACCEPT` — 로컬 테스트 전용 |

### 프리미엄 MCP 툴

| Tool | 입력 | 동작 |
|------|------|------|
| `compare_tools` | `slugs[]` (2–4) | trust/x402/chains 비교 표 |
| `export_toolkit` | `slugs[]` 또는 `category` | JSON + markdown 설치 번들 |

### 결제 플로우

```
Agent → POST /mcp tools/call compare_tools (no PAYMENT-SIGNATURE)
  ← HTTP 402 + PAYMENT-REQUIRED (base64 x402 v2 JSON)

Agent wallet signs → POST /mcp + PAYMENT-SIGNATURE
  ← HTTP 200 + JSON-RPC result
```

검증: 구조적 `x402Version: 2` 확인 후, `X402_FACILITATOR_URL/verify`로 settlement 확인(설정 시). OnchainAI는 자금을 만지지 않는다.

---

## 3. §Compliance — 허용 vs 금지

### ✅ 허용 (Allowed)

| 항목 | 설명 |
|------|------|
| **축 B — 자기 MCP 판매** | OnchainAI pay-to 주소로 `compare_tools` / `export_toolkit` 호출당 x402 수취 |
| **HTTP 402 노출** | `POST /mcp`에서 `PAYMENT-REQUIRED` / `PAYMENT-SIGNATURE` 표준 헤더 사용 |
| **셀프서비스 설정** | Admin이 premium enabled, pay-to, price, network, display price 편집 |
| **축 A — 카탈로그 메타** | 제3자 x402 툴의 `x402_price`, referral 메타, 검증 플래그 **공개** (기존 `X402_REFERRAL_SPEC`) |
| **외부 facilitator 검증** | `X402_FACILITATOR_URL`로 verify만 호출 — 커스터디·프록시 아님 |
| **기본값 off** | `mcp_premium_enabled = false` → 전 MCP 무료 (하위 호환) |

### ❌ 금지 (Forbidden)

| 항목 | 이유 |
|------|------|
| **커스터디** | 유저/제공자 USDC를 OnchainAI 지갑에 보관·정산 |
| **제3자 결제 프록시/게이트웨이** | 등록된 x402 툴 호출을 OnchainAI가 래핑·마진·중계 (MVP_DESIGN “라우팅 수수료” 범위 밖) |
| **MCP에서 x402 숨김** | 402를 JSON-RPC 200 error로만 반환하고 HTTP 402/`PAYMENT-REQUIRED` 생략 |
| **미문서 `referrer` / `split` 필드** | 결제 요청에 invent 금지 (`AGENTS.md` Hard Rules) |
| **검증 플래그를 public gate에 추가** | `payment_verified` 등으로 노출 차단 금지 |
| **클라이언트에 pay-to 노출 (공개 API)** | `sanitize_site_settings_for_public`로 premium pay-to strip 유지 |
| **제3자 자금 “판매”** | 디렉터리가 타인 x402 엔드포인트 대금을 대신 수취·배분 |

### 경계 한 줄

> **카탈로그의 x402 = 메타·신뢰.** **OnchainAI MCP의 x402 = 자기 서비스 대가.** 둘 다 **결제 경로에 끼어 제3자 돈을 만지지 않는다.**

---

## 4. AGENTS.md 반영 사항

`AGENTS.md`에 추가·수정할 내용:

1. **Topic Routing** 한 줄: `MCP premium x402 (Axis B): docs/MCP_X402_MONETIZATION_SPEC.md`
2. **Hard Rules** x402 줄 정밀화:
   - 제3자 카탈로그 x402: 메타/어트리뷰션만
   - **예외**: OnchainAI **자기** `POST /mcp` premium은 HTTP 402 허용 (`compare_tools`, `export_toolkit`)
   - 여전히 금지: custody, third-party proxy gateway, undocumented `referrer`/`split`

---

## 5. 수용 기준

- [ ] `mcp_premium_enabled=false` → premium 툴 무료 동작
- [ ] `mcp_premium_enabled=true` + pay-to/price 설정 → unpaid `compare_tools` → **HTTP 402** + `PAYMENT-REQUIRED`
- [ ] `tools/list`에 `compare_tools`, `export_toolkit` 노출 (7 public tools)
- [ ] Public `GET /api/v2/settings`에 `mcp_premium_pay_to_address` 미노출
- [ ] `cargo test --features ssr` 그린

---

## 6. 검증

```bash
cargo test --features ssr mcp_x402
cargo test --features ssr mcp_premium
cargo check --features ssr
```

수동 (premium enabled 후):

```bash
curl -i -X POST 'http://localhost:3000/mcp' \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"compare_tools","arguments":{"slugs":["a","b"]}}}'
# Expect: HTTP/1.1 402 + PAYMENT-REQUIRED header
```