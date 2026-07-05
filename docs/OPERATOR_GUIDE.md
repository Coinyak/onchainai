# OnchainAI 운영 설명서

운영자(`is_admin = true`)·개발자·일반 유저가 각각 무엇을 할 수 있는지 정리한 문서.

---

## 1. 코딩 없이 가능 — 운영자 어드민 패널 (`/admin/*`)

`is_admin = true` 계정으로 로그인하면 접근. 모든 동작은 서버에서 권한 재확인(`check_admin_access`).

### `/admin` — Operator Dashboard
- 리뷰 대기 큐 개수, 크롤러 소스 상태(health), 각 섹션 바로가기 확인.
- 읽기 전용 현황판.

### `/admin/tools` — Review Queues (도구 심사)
- 크롤러가 자동 발견한 도구를 **승인(Approve) / 거부(Reject)**.
- relevance(관련성) + install safety(설치 위험도) 기준으로 분리된 큐.
- 거부 시 사유(reason) 입력. 운영자 오버라이드로 강제 승인 가능.
- 승인/거부 = 공개 노출 여부 결정.

### `/admin/featured` — Featured Carousel (홈 추천 카드)
- 홈 히어로 아래 노출되는 하이라이트 카드 **추가 / 수정 / 삭제**.
- 도구 검색해서 연결, 카드 이미지 업로드, 카피 작성.
- **로컬 개발 시드:** `seeds/dev_seed_featured.sql` (1–3장 카드). **프로덕션:** 이 패널에서 직접 추가하거나 동일 데이터를 시드.

### `/admin/settings` — Site Settings (사이트 설정)
코딩 없이 바꿀 수 있는 **유일한 "텍스트/정책" 조정 지점**:
- Site name (사이트 이름)
- Slogan (슬로건)
- Description (설명)
- MCP endpoint (MCP 엔드포인트 URL)
- Search keywords (크롤러 검색 키워드)
- 토글 스위치:
  - Allow free registration (무료 등록 허용)
  - Require approval for new tools (신규 도구 승인 필수)
  - Allow x402 paid registration (x402 유료 등록 허용)

### `/admin/crawler` — Crawler Control (크롤러 제어)
- **7개 발견 소스** 상태/마지막 실행 시각 확인: `cryptoskill`, `github`, `mcp-registry`, `npm`, `vendor_orgs`, `bazaar`, `web3-mcp-hub` (`CRAWLER_SOURCE_DEFS`와 동기화).
- 소스별 **수동 크롤 즉시 실행(Trigger)**. (자동 스케줄과 별개) — UI가 UUID를 보내는 버그가 있으면 §5 curl 우회.
- **GitHub Stars Sync** (`sync_stars`) — 별도 유지보수 작업; Stars Sync Now 버튼.

**발견 소스 구분 (혼동 방지):**

| 소스 | 무엇을 긁나 | 주기 | 심사 |
|------|-------------|------|------|
| `github` | GitHub **topic** 검색 (`mcp-server`, `crypto-mcp`, …) | 1h | 사이트 설정 따름 |
| `vendor_orgs` | `scripts/vendor-orgs.json` **curated org** repo sweep (Circle, Coinbase, …) | 일 03:45 UTC | **항상 pending** |
| `bazaar` | Coinbase CDP **x402 API** discovery (GitHub 아님) | 6h | **항상 pending** |

### `/admin/categories` — Category Management
- 기능 카테고리 **생성 / 수정 / 삭제** (CRUD).

### `/admin/users` — User Management
- Ban / Unban (정지·해제)
- Make Admin / Remove Admin (관리자 권한 부여·회수)
- Delete (계정 삭제)

### `/admin/comments` — Comment Moderation
- 댓글 삭제.
- 댓글 삭제 + 작성자 동시 정지.

### 코딩 없이 가능한 UI/UX 수정 = **제한적**
바꿀 수 있는 것: 사이트 이름·슬로건·설명·MCP 엔드포인트(settings), 추천 카드 이미지·문구(featured), 카테고리 이름.
**못 바꾸는 것: 레이아웃·색상·폰트·컴포넌트·페이지 구조 = 전부 코드.** (아래 2번)

---

## 2. 코딩으로만 가능 — 개발자 작업

| 영역 | 위치 | 내용 |
|------|------|------|
| 페이지 레이아웃·색상·폰트·컴포넌트 | `src/components/`, `src/pages/`, `style/`, `DESIGN.md` | 디자인 토큰, Tailwind, Leptos 컴포넌트. settings로 못 바꾸는 모든 시각 요소 |
| 새 페이지 / 라우트 | `src/app.rs` | 새 URL 경로 추가 |
| 새 어드민 기능 / 새 설정 항목 | `src/pages/admin/`, `src/server/functions.rs` | settings에 없는 새 토글·필드 추가 |
| 크롤러 소스 추가·변경 | `src/crawler/sources/` | 7개 발견 소스: `cryptoskill`, `github`, `mcp-registry`, `npm`, `vendor_orgs`, `bazaar`, `web3-mcp-hub` + `sync_stars` 유지보수 |
| 크롤러 스케줄 변경 | `src/crawler/scheduler.rs` | cron (`CRAWLER_JOB_SPECS`): npm·github 1h, cryptoskill·bazaar 6h, mcp-registry·web3-mcp-hub 12h, vendor_orgs 일 03:45 UTC, sync_stars 30m |
| 관련성/설치안전 판정 로직 | `src/crawler/relevance.rs`, `src/install_safety.rs` | 자동 심사 기준 |
| DB 스키마 | `migrations/` | 테이블·컬럼 변경 후 `sqlx migrate run` + `sqlx prepare` |
| MCP 서버 도구 | `src/server/mcp.rs` | 에이전트용 MCP 4개 도구 |
| 인증 흐름 | `src/auth/` | GitHub / email / SIWX(지갑) 로그인 |
| 레이트 리밋·보안 헤더 | `src/server/rate_limit.rs`, `docs/SECURITY.md` | |
| 배포 | `scripts/`, `docs/BUILD_DEPLOY_RULES.md` | 빌드·Railway 배포 |

### 배포 후 검증 (개발자·운영자 공통)
Railway 배포 후 회귀 확인:
```bash
./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz
node scripts/click-test.mjs https://www.onchain-ai.xyz
```
`post-deploy-verify.sh`는 curl smoke + `browser-smoke.mjs` + `click-test.mjs`를 실행합니다. load-more·`?page=2` 누적 카드 수(50→100) 실패 시 배포 롤백을 검토하세요.
UI 변경 검수용 스크린샷은 로컬 또는 배포 URL에 대해 `node scripts/visual-snapshots.mjs <url> --out .playwright-cli/ui-snapshots`로 캡처합니다.

### 공개 카탈로그 품질 (요약)
- 공개 목록: `approval_status=approved`, `relevance_status=accepted`, critical install risk·quarantine 제외.
- 상세: `docs/UI_UX_DESIGN.md` §12.1.2.

핵심: **데이터(도구/카드/카테고리/유저/설정 값)는 운영자가 손댐. 동작·모양·구조는 개발자가 손댐.**

---

## 3. 유저 활동

### 비로그인 (공개) — 인증 불필요
- 도구 탐색 (`/tools`), 검색·필터 (체인별, 카테고리별)
- 도구 상세 보기 (`/tools/:slug`), 댓글 읽기
- 카테고리 페이지 (`/categories/:id`)
- About (`/about`)
- 에이전트는 MCP 엔드포인트로 도구 조회 가능

### 로그인 필요
로그인 방식 3종: **GitHub OAuth / 이메일 / SIWX(지갑 서명)**

- 도구 제출 (`/submit`) — 신규 크립토 도구 등록 신청 (`submit_tool`), 무료 또는 x402 유료
- 내 제출 목록 (`list_my_submissions`)
- 댓글 작성 (`create_comment`)
- 업보트 (`toggle_upvote`)
- 북마크 (`toggle_bookmark`)
- 도구 신고 (`report_tool`)
- 도구 소유권 클레임 신청 (`request_tool_claim`)
- 온보딩 프로필 설정 (`/onboarding/profile`)

규칙: **도구 탐색은 공개. 댓글·업보트·북마크·제출은 로그인 필수.**

---

## 4. 자동 검증 하네스 — verified/official 상태 (AI·운영자 공용)

"이 도구 verified/official 처리해줘"라는 요청은 사람이 판단하지 말고 **하네스를 실행**한다.
어떤 AI 에이전트든 같은 명령으로 같은 결과가 나온다 (증거 기반, deny-by-default).

```bash
# 드라이런(기본): 판정·증거만 출력, 쓰기 없음
node scripts/verify-tool-official.mjs <slug>

# 자동 승인: 증거가 기준을 넘으면 status 갱신 + tool_review_events 감사 기록
node scripts/verify-tool-official.mjs <slug> --apply

# 전체 스캔(공개 도구 중 first-party org / 플랫폼 키워드 후보 일괄 판정)
node scripts/verify-tool-official.mjs --scan [--apply --i-understand-bulk] [--limit N]
```

- **일괄 적용**: `--scan --apply`는 반드시 `--i-understand-bulk`와 함께. 없으면 스크립트가 거부(deny-by-default).
- **판정 규칙**: `official` = repo org가 first-party 목록(스크립트 상단 `FIRST_PARTY_ORGS`,
  PR로 확장)이거나 GitHub 도메인 인증 org + org 사이트가 도구 홈페이지와 일치.
  `verified` = repo 실존 + 아이덴티티 클러스터(org/npm scope/홈페이지 도메인) 일치.
  증거 부족이면 community 유지. **다운그레이드는 절대 안 함.** 공개 게이트
  (승인/관련성/critical 위험/격리) 미통과 도구는 승격 거부.
- **감사**: 모든 적용은 `tool_review_events`(action=`agent_auto_status`)에 before/after와
  근거가 남는다. x402 결제 플래그(`payment_verified` 등)는 이 하네스가 건드리지 않는다
  (소유권 증명은 클레임 플로 + 운영자).
- **환경**: 레포 루트 `.env`(또는 `ENV_FILE=경로`)의 `SUPABASE_URL`+`SUPABASE_SERVICE_KEY`
  또는 `DATABASE_URL`. Supabase REST(PostgREST)가 503(PGRST002)이면 자동으로 직접
  Postgres 폴백 — 최초 1회 `npm install --prefix scripts/ops` 필요.

---

## 5. Link-drop — 카탈로그 공급 4경로 (운영자 러너북)

크롤러·시드·검증 하네스로 **새 도구를 카탈로그에 넣는** 운영 절차. 코드 변경 없이 아래만 따른다.

### 5.1 크롤러 발견 → `/admin/tools` 심사 큐

1. 7개 발견 소스가 주기적으로 upsert (`src/crawler/scheduler.rs`).
2. **`vendor_orgs`**·**`bazaar`** 소스는 `persist_crawl_results_gated`로 **항상 `approval_status=pending`** — `require_tool_approval` 설정과 무관.
3. 운영자: `/admin/tools`에서 relevance·install safety 큐를 검토하고 Approve/Reject.
4. 수동 1회 실행이 필요하면 §5.4 curl 사용 (UI Trigger 버그 우회).

### 5.2 운영자 큐레이션 시드 (`SEED_ENV=prod-curate`)

SQL `dev_seed.sql`과 **별도 레인**. `scripts/seed-tool-lib.mjs` 기반 `scripts/seed-*.mjs`가 프로덕션에 operator-curated upsert.

```bash
# 1) 드라이런(기본): SEED_ENV 미설정 또는 prod-curate 아님 → JSON 요약만, DB 쓰기 없음
ENV_FILE=.env node scripts/seed-circle-agent-tools.mjs

# 2) 적용: 명시적 prod-curate + .env의 DATABASE_URL(프로덕션)
ENV_FILE=.env SEED_ENV=prod-curate PG_INSECURE_SSL=1 node scripts/seed-circle-agent-tools.mjs
```

- Circle 11 slugs 예: `circle-agent-stack`, `circle-gateway`, `circle-cctp-v2`, … (`scripts/seed-circle-agent-tools.mjs` 참고).
- 다른 스크립트: `seed-onchainai-listing.mjs`, `seed-platform-agent-tools.mjs`, `seed-crypto-infra-tools.mjs` 등 — 모두 동일 dry-run → `prod-curate` 패턴.
- 상세: `docs/SEED_DATA.md` §7.1.

### 5.3 verified/official 승격 (단일 + 일괄)

§4 하네스. 시드·크롤 후 repo/org 증거가 있으면:

```bash
# 단일 슬러그
node scripts/verify-tool-official.mjs circle-gateway --apply

# 일괄 스캔 + 적용 (bulk ack 필수)
node scripts/verify-tool-official.mjs --scan --apply --i-understand-bulk
```

수동 `tools.status` 변경 금지 — 하네스만 사용.

### 5.4 크롤러 수동 Trigger — curl 우회 (UI UUID 버그)

`/admin/crawler` Trigger가 소스 UUID 대신 **소스 이름**을 보내야 할 때까지:

```bash
API_URL=https://www.onchain-ai.xyz   # 또는 로컬 API
# 관리자 세션 쿠키 필요 (브라우저에서 복사)

curl -sS -X POST "$API_URL/api/v2/admin/crawler/trigger" \
  -H "Cookie: $ADMIN_COOKIE" \
  -H "Content-Type: application/json" \
  -d '{"source":"bazaar"}'

# vendor org sweep:
curl -sS -X POST "$API_URL/api/v2/admin/crawler/trigger" \
  -H "Cookie: $ADMIN_COOKIE" \
  -H "Content-Type: application/json" \
  -d '{"source":"vendor_orgs"}'
```

허용 `source` 값: `npm`, `cryptoskill`, `web3-mcp-hub`, `github`, `mcp-registry`, `vendor_orgs`, `bazaar`, `sync_stars` (`validate_trigger_crawler_source`).

---

## 요약 3줄
1. **운영자(노코드)**: 도구 승인/거부, 추천카드, 카테고리, 유저 정지, 댓글 삭제, 사이트 텍스트·MCP·크롤러 키워드·등록정책 토글, 크롤러 수동 실행, §5 link-drop(시드·verify·curl).
2. **개발자(코드)**: 화면 모양·레이아웃·새 페이지·크롤러 소스/주기·판정 로직·DB·MCP·인증·배포.
3. **유저**: 탐색·검색은 무료 공개 / 댓글·투표·북마크·제출·신고는 로그인(GitHub·이메일·지갑).
