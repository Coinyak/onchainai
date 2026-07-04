# Agent-Native Operations Spec

> **전제:** 코딩 에이전트가 구축·배포하고, 운영자가 웹사이트 내 어드민으로 기능을 관리한다.
> UI 변경이 잦고, 기능 추가도 많고, 빠른 정식 오픈이 목표다.
> 모든 설계 결정은 이 조건에서 출발한다.

---

## 핵심 결정: 프론트엔드 분리

### 왜 분리하는가

| 조건 | Leptos 현 구조 | 분리 후 |
|------|---------------|---------|
| UI 변경 | Rust 컴파일 + WASM 빌드 + 일관성 검증 (75초+) | HMR sub-second |
| 기능 추가 | 컴포넌트 + server fn + WASM 동시 빌드 | API 연결만, 프론트는 즉시 |
| 에이전트 코드 생성 | Leptos 학습 데이터 부족, 잦은 오류 | React/Next.js는 LLM이 가장 잘 생성 |
| 배포 | Railway Docker (Rust 풀 빌드 25~45분) | Vercel (프론트 push 즉시) + Railway (API만) |
| 정식 오픈 속도 | UI iteration 주기가 느려 출시 지연 | 프론트 iteration 주기가 빨라 출시 단축 |

### 무엇을 버리고 무엇을 지키는가

| 레이어 | 현재 | 분리 후 | 줄 수 | 비고 |
|--------|------|---------|-------|------|
| MCP 5 도구 | Rust | **Rust 유지** | ~1,200줄 | 에이전트 가치의 핵심, 안 건드림 |
| 크롤러/normalizer/deduper | Rust | **Rust 유지** | 5,284줄 | 데이터 파이프라인, 안 건드림 |
| install_safety | Rust | **Rust 유지** | 486줄 | 위험 판정, 안 건드림 |
| public_tool_where!() + 쿼리 | Rust | **Rust 유지** | ~1,500줄 | 가시성 규칙, 안 건드림 |
| 인증 (GitHub/Email/SIWX) | Rust | **Rust 유지** | 2,333줄 | 세션/JWT/SIWX 검증, 안 건드림 |
| 서버 함수 60개 | Leptos server fn | **Axum JSON API** | ~5,000줄 | 로직은 유지, 프로토콜만 교체 |
| 어드민 비즈니스 로직 | Leptos server fn | **Axum JSON API** | (포함) | 권한 확인, DB 조작 유지 |
| UI 컴포넌트 34개 | Leptos | **Next.js React** | 6,648줄 | 재작성 |
| 페이지 20개 | Leptos | **Next.js React** | 3,801줄 | 재작성 |
| CSS | 수작업 output.css | **Tailwind CSS** | 2,193줄 | 토큰 기반 재구성 |

**버리는 것:** Leptos 컴포넌트/페이지/CSS (~12,700줄).
**지키는 것:** 백엔드 전체 (~19,000줄) — MCP, 크롤러, 인증, 쿼리, install_safety, 비즈니스 로직.

---

## A. 백엔드: Leptos server fn → Axum JSON API

### A1. 전환 원칙

기존 `#[server(FunctionName, "/api")]` 함수들을 Axum 핸들러로 변환. 로직(쿼리, 검증, 권한 확인)은 그대로, 입출력 프로토콜만 교체.

```
기존: #[server(SearchTools, "/api")]
      pub async fn search_tools(...) -> Result<Vec<Tool>, ServerFnError>

이후: async fn search_tools(State(state): State<AppState>, Json(req): Json<SearchReq>)
           -> Result<Json<SearchRes>, ApiError>
```

### A2. API 엔드포인트 맵 (60개 server fn → REST)

**공개 카탈로그**
| Method | Path | 기존 server fn | 용도 |
|--------|------|---------------|------|
| GET | `/api/tools` | ListTools, ListToolsV1 | 도구 목록 (필터/페이지) |
| GET | `/api/tools/search` | SearchTools | 텍스트 검색 |
| GET | `/api/tools/:slug` | GetToolBySlug | 도구 상세 |
| GET | `/api/tools/recent` | GetRecentTools | 최신 도구 |
| GET | `/api/tools/count` | CountTools | 도구 수 |
| GET | `/api/tools/compare` | CompareTools | 도구 비교 |
| GET | `/api/tools/comment-counts` | GetToolCommentCounts | 댓글 수 |
| GET | `/api/browser-data` | LoadBrowserData | 브라우저 페이지 통합 로드 |
| GET | `/api/dashboard` | GetPublicDashboardSnapshot | 공개 대시보드 |
| GET | `/api/categories` | GetCategories | 카테고리 목록 |
| GET | `/api/chains` | GetChainCounts | 체인별 도구 수 |
| GET | `/api/toolkit` | ListMyToolkit | 내 툴킷 |
| PUT | `/api/toolkit/:slug` | UpdateToolkitItem | 툴킷 아이템 업데이트 |

**인증**
| Method | Path | 기존 | 용도 |
|--------|------|------|------|
| GET | `/auth/github` | routes::github_login | GitHub OAuth 시작 |
| POST | `/auth/github/switch` | routes::github_switch | 계정 전환 |
| POST | `/auth/email` | email::send_magic_link | 매직링크 발송 |
| GET | `/auth/callback` | routes::oauth_callback | OAuth 콜백 |
| POST | `/auth/logout` | routes::logout | 로그아웃 |
| POST | `/auth/siwx/challenge` | siwx::challenge | SIWX 챌린지 |
| POST | `/auth/siwx/verify` | siwx::verify | SIWX 검증 |
| POST | `/onboarding/complete` | onboarding::complete | 온보딩 완료 |
| POST | `/onboarding/skip` | onboarding::skip | 온보딩 스킵 |
| GET | `/api/me` | GetCurrentUser | 현재 사용자 |
| GET | `/api/admin/check` | CheckAdminAccess | 관리자 확인 |

**사용자 활동**
| Method | Path | 기존 | 용도 |
|--------|------|------|------|
| GET | `/api/tools/:slug/comments` | GetToolComments | 댓글 목록 |
| POST | `/api/tools/:slug/comments` | CreateComment | 댓글 작성 |
| POST | `/api/comments/:id/upvote` | ToggleUpvote | 업보트 |
| GET | `/api/tools/:slug/bookmark` | IsBookmarked | 북마크 확인 |
| PUT | `/api/tools/:slug/bookmark` | SetBookmark | 북마크 설정 |
| POST | `/api/tools/:slug/bookmark` | ToggleBookmark | 북마크 토글 |
| POST | `/api/tools/:slug/report` | ReportTool | 신고 |
| POST | `/api/tools/:slug/claim` | RequestToolClaim | 소유권 클레임 |
| POST | `/api/submit` | SubmitTool | 도구 등록 |
| GET | `/api/my-submissions` | ListMySubmissions | 내 제출 목록 |

**어드민 — 도구 심사**
| Method | Path | 기존 | 용도 |
|--------|------|------|------|
| GET | `/api/admin/pending` | ListPendingTools | 대기 큐 |
| GET | `/api/admin/stats` | GetAdminDashboardStats | 대시보드 통계 |
| GET | `/api/admin/review-queue` | ListReviewQueue | 리뷰 큐 |
| POST | `/api/admin/review` | ReviewTool | 승인/거부 |
| POST | `/api/admin/approval` | SetToolApproval | 승인 상태 |
| GET | `/api/admin/referral-stats` | GetReferralDashboardStats | 추천 통계 |
| PUT | `/api/admin/tool-referral` | UpdateToolReferral | 추천 설정 |
| GET | `/api/admin/workbench/summary` | GetAdminWorkbenchSummary | 워크벤치 요약 |
| GET | `/api/admin/workbench/:slug` | GetAdminToolWorkbench | 도구 워크벤치 |
| GET | `/api/admin/trust/:slug` | GetToolTrustView | 트러스트 뷰 |
| POST | `/api/admin/verify-link` | VerifyToolOfficialLink | 공식 링크 확인 |

**어드민 — 분류/추천**
| Method | Path | 기존 | 용도 |
|--------|------|------|------|
| GET | `/api/featured` | GetFeaturedCards | 공개 추천 카드 |
| GET | `/api/admin/featured` | ListFeaturedCards | 추천 카드 관리 |
| POST | `/api/admin/featured` | CreateFeaturedCard | 추천 카드 생성 |
| PUT | `/api/admin/featured/:id` | UpdateFeaturedCard | 추천 카드 수정 |
| DELETE | `/api/admin/featured/:id` | DeleteFeaturedCard | 추천 카드 삭제 |
| POST | `/api/admin/featured/upload` | UploadFeaturedImage | 이미지 업로드 |
| GET | `/api/admin/featured/search` | SearchToolsForPicker | 도구 검색 (피커용) |
| GET | `/api/admin/categories` | ListAdminCategories | 카테고리 관리 |
| POST | `/api/admin/categories` | CreateCategory | 카테고리 생성 |
| PUT | `/api/admin/categories/:id` | UpdateCategory | 카테고리 수정 |
| DELETE | `/api/admin/categories/:id` | DeleteCategory | 카테고리 삭제 |

**어드민 — 사용자/댓글/크롤러/설정**
| Method | Path | 기존 | 용도 |
|--------|------|------|------|
| GET | `/api/admin/users` | ListAdminUsers | 사용자 목록 |
| PUT | `/api/admin/users/:id/ban` | SetUserBanned | 정지/해제 |
| PUT | `/api/admin/users/:id/admin` | SetUserAdmin | 관리자 권한 |
| DELETE | `/api/admin/users/:id` | DeleteUser | 계정 삭제 |
| GET | `/api/admin/comments` | ListAdminComments | 댓글 목록 |
| DELETE | `/api/admin/comments/:id` | DeleteAdminComment | 댓글 삭제 |
| DELETE | `/api/admin/comments/:id/ban-author` | DeleteCommentAndBanUser | 댓글 삭제+정지 |
| GET | `/api/admin/crawler/sources` | ListCrawlerSources | 크롤러 소스 |
| POST | `/api/admin/crawler/trigger` | TriggerCrawlerSource | 수동 크롤 |
| GET | `/api/settings` | GetSiteSettings | 공개 설정 |
| GET | `/api/admin/settings` | GetAdminSiteSettings | 관리자 설정 |
| PUT | `/api/admin/settings` | UpdateSiteSettings | 설정 업데이트 |

**MCP + Operator (이미 Axum 핸들러, 변경 없음)**
| Method | Path | 용도 |
|--------|------|------|
| POST | `/mcp` | MCP JSON-RPC (5 도구) |
| GET | `/api/admin/operator/snapshot` | 운영자 스냅샷 |
| POST | `/api/admin/operator/run` | 운영자 실행 |
| POST | `/api/admin/operator/review-run` | 리뷰 실행 생성 |
| POST | `/api/admin/operator/review-entry` | 리뷰 엔트리 추가 |
| GET | `/api/admin/operator/review-timeline` | 리뷰 타임라인 |

### A3. 인증 토큰 전달

기존: Leptos server fn는 쿠키를 자동 읽음.
이후: Next.js가 쿠키를 포함해 API 호출 (credentials: 'include'). Axum 핸들러가 기존 auth 미들웨어로 동일하게 쿠키 검증. 세션/JWT/SIWX 로직은 변경 없음.

### A4. 에러 응답 표준

```rust
// 모든 API 핸들러의 에러 응답
{
  "error": {
    "code": "not_found",       // machine-readable
    "message": "Tool not found: uniswap-v3"  // human-readable
  }
}
```

기존 ServerFnError를 ApiError enum으로 매핑.

---

## B. 프론트엔드: Next.js + React

### B1. 기술 스택

- **Next.js 15** (App Router, SSR)
- **React 19**
- **Tailwind CSS v4** (DESIGN.md 토큰을 Tailwind config로 변환)
- **TypeScript**
- **Vercel 배포** (프론트), Railway 배포 (API)

### B2. 디자인 토큰 마이그레이션

DESIGN.md의 토큰을 Tailwind config로 직접 매핑:

```js
// tailwind.config.js
colors: {
  primary: '#1A1A1A',
  secondary: '#6B6B6B',
  tertiary: '#E76F00',
  'neutral-bg': '#FFFFFF',
  'neutral-surface': '#F5F5F0',
  'neutral-hover': '#FAFAFA',
  border: '#E5E5E5',
  'border-strong': '#D1D1D1',
  'text-muted': '#999999',
  error: '#C0392B',
  success: '#2D7D46',
}
// borderRadius, spacing, fontSize 동일하게 매핑
```

### B3. 페이지 매핑 (20개)

| Leptos 페이지 | Next.js 라우트 | 우선순위 |
|--------------|---------------|---------|
| tools_browser | `/tools` | P0 |
| tool_detail | `/tools/[slug]` | P0 |
| home | `/` | P0 |
| categories | `/categories/[id]` | P1 |
| about | `/about` | P2 |
| submit | `/submit` | P1 |
| onboarding/profile | `/onboarding/profile` | P1 |
| login | `/login` | P0 |
| admin/dashboard | `/admin` | P1 |
| admin/tools | `/admin/tools` | P1 |
| admin/featured | `/admin/featured` | P1 |
| admin/categories | `/admin/categories` | P1 |
| admin/settings | `/admin/settings` | P1 |
| admin/users | `/admin/users` | P1 |
| admin/comments | `/admin/comments` | P1 |
| admin/crawler | `/admin/crawler` | P1 |

### B4. 컴포넌트 매핑 (34개 → React)

거대 파일 우선 분해:
- `tools_browser.rs` (1,400줄) → `ToolsBrowser` + `ToolList` + `FilterPanel` + `Pagination` 컴포넌트로 분리
- `sidebar.rs` (800줄) → `Sidebar` + `FilterChip` + `CategoryList` + `ChainList` 분리
- `tool_detail_content.rs` (800줄) → `ToolDetail` + `InstallSection` + `TrustFacts` + `CommentSection` 분리

각 컴포넌트를 200줄 이하로 유지. 에이전트가 더 잘 생성하고, 변경 범위가 좁아진다.

### B5. 상태 관리

- 서버 상태: TanStack Query (캐싱, 재시도, 로딩 상태)
- 클라이언트 상태: React useState/useReducer (필터, 모달, 사이드바)
- 인증 상태: React Context + 쿠키 (서버 API가 쿠키 검증)

---

## C. MCP 도구 응답 품질 개선 (분리와 무관, 즉시 실행 가능)

### C1. description 보강 (P0)

| 도구 | 개선 |
|------|------|
| `get_tool_detail` | "Get full detail (trust score, install risk, x402 status, chains, repo) for a tool by slug. Use the slug from search_tools. Call before get_install_guide to verify trust and risk." |
| `list_categories` | "List all categories with tool counts. Use the returned id as search_tools category filter to browse by function." |
| `get_install_guide` | "Get platform-specific install guide. Pass slug from search_tools/detail and platform (claude, cursor, generic). If blocked=true or risk_level=critical, do not install." |

### C2. category enum 동기화 (P1)

`search_tools` inputSchema category에 14개 enum 추가. `validate_category`를 DB 기반으로 전환.

### C3. cursor description 수정 (P1)

"Opaque pagination cursor" → "Pagination offset string from previous next_cursor. Starts at 0."

### C4. search 응답 slim DTO + total_count + compact JSON (P2)

search 응답을 McpToolSummary(10 필드)로 줄이고, total_count/has_more 추가, to_string_pretty → compact.

---

## D. 어드민 패널 확장 (운영자가 웹에서 관리)

### D1. 콘텐츠 관리 (P1)

새 `/admin/content` 패널: 헤더 로고, 푸터 링크, About 본문, 홈 히어로 카피, 빈 상태 메시지.

```sql
ALTER TABLE site_settings
  ADD COLUMN hero_title TEXT,
  ADD COLUMN hero_subtitle TEXT,
  ADD COLUMN about_content TEXT,
  ADD COLUMN footer_links JSONB DEFAULT '[]';
```

### D2. 크롤러 스케줄 관리 (P2)

```sql
ALTER TABLE crawler_sources
  ADD COLUMN schedule_minutes INT NOT NULL DEFAULT 360,
  ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT true;
```

### D3. 구현 원칙

- Next.js 어드민 페이지가 API로 settings를 읽어 폼 렌더.
- 모든 변경은 서버 측 `check_admin_access` 재확인.
- settings 값이 없으면 기본값 fallback (사이트가 깨지지 않도록).
- Leptos 하드코딩을 DB 기반으로 전환하는 작업은 프론트 분리와 함께 진행.

---

## E. 배포 구조

### E1. 이중 배포

| 서비스 | 역할 | 빌드 | 배포 |
|--------|------|------|------|
| Railway (Rust) | API + MCP + 인증 + 크롤러 | cargo build --features ssr | git push main → 자동 |
| Vercel (Next.js) | 프론트엔드 | next build | git push main → 자동 |

### E2. 환경 변수

- Railway: DB, Supabase, JWT, GitHub OAuth (기존과 동일)
- Vercel: `NEXT_PUBLIC_API_URL=https://www.onchain-ai.xyz`
- CORS: Axum이 이미 Vercel 도메인 + localhost 허용 (lib.rs CorsLayer)

### E3. Railway Dockerfile 단순화 (Leptos 제거)

분리 후 Dockerfile에서 WASM 빌드가 사라진다. cargo-chef는 여전히 권장:

```dockerfile
FROM rust:1.90-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features ssr --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ migrations/
RUN cargo build --release --features ssr

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/onchainai /app/onchainai
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/public /app/public
CMD ["/app/onchainai"]
```

WASM/cargo-leptos/cargo-leptos 의존성 제거. 빌드 시간 추가 단축.

---

## F. 마이그레이션 순서

### Phase 1: API 전환 (백엔드, Leptos 유지하며 병행)

1. 기존 server fn 로직을 Axum 핸들러로 복제 (server fn는 유지)
2. 각 핸들러에 대해 기존 로직(쿼리, 검증, 권한) 재사용
3. `/api/v2/*` 경로로 노출 (기존 `/api/*` Leptos server fn와 충돌 회피)
4. 통합 테스트: 기존 server fn 결과와 Axum 핸들러 결과가 동일한지 검증
5. MCP 도구 description 보강 (C1~C3) 동시 진행

### Phase 2: 프론트 구축 (Next.js, 병행)

1. Tailwind config + 디자인 토큰 매핑
2. P0 페이지: home, tools, tool detail, login
3. P0 컴포넌트: ToolCard, SearchBar, Sidebar, ToolDetail, BottomSheet
4. TanStack Query로 API 연결
5. 인증: 쿠키 기반 세션 연결

### Phase 3: 전환

1. P1 페이지: admin 전체, submit, onboarding, categories
2. Vercel 배포 + Railway API 배포
3. DNS 전환 (www.onchain-ai.xyz → Vercel, api.onchain-ai.xyz → Railway 또는 경로 기반 분리)
4. Leptos server fn 제거, `/api/v2/*`를 `/api/*`로 전환
5. Leptos 컴포넌트/페이지/CSS 제거
6. cargo-leptos 의존성 제거, Dockerfile 단순화

### Phase 4: 어드민 확장

1. 콘텐츠 관리 패널 (D1)
2. 크롤러 스케줄 관리 (D2)
3. MCP search slim DTO + total_count (C4)

### 병행 가능 작업 (Phase 1과 동시)

- MCP description 보강 (C1~C3): Rust만, 프론트 무관, 즉시
- 어드민 콘텐츠 관리 마이그레이션 (D1): API 전환과 함께
- Dockerfile cargo-chef (E3): API 전환과 함께

---

## G. 에이전트 워크플로 (분리 후)

### UI 변경 시

1. 코드 수정 (Next.js 컴포넌트)
2. HMR로 즉시 확인 (sub-second)
3. `next lint && next build` (커밋 전)
4. 커밋 + main 머지 → Vercel 자동 배포 (1~2분)

### API/MCP 변경 시

1. `cargo check --features ssr` (빠른 확인)
2. 코드 수정
3. `cargo test --features ssr` (커밋 전)
4. 커밋 + main 머지 → Railway 자동 배포 (cargo-chef로 8~15분)

### 어드민 기능 추가 시

1. API 핸들러 추가 (Rust)
2. Next.js 어드민 페이지 추가 (React)
3. 각각 독립적으로 빌드/배포

### 핵심: UI와 API가 독립적으로 변경·배포된다

더 이상 UI 변경을 위해 Rust를 컴파일하지 않는다. 더 이상 API 변경이 WASM 번들 일관성을 깨뜨리지 않는다.

---

## H. 검증 체크리스트

### API 변경 시
- [ ] `cargo test --features ssr` 통과
- [ ] 기존 server fn 결과와 동일한 응답 (Phase 1 병행 기간)
- [ ] MCP visibility 필터 테스트 통과
- [ ] 인증 미들웨어 동작 확인

### 프론트 변경 시
- [ ] `next lint && next build` 통과
- [ ] DESIGN.md 토큰 준수 (Tailwind config)
- [ ] 기존 data-testid 보존
- [ ] 모바일 반응형 확인

### 어드민 확장 시
- [ ] 마이그레이션 + sqlx prepare
- [ ] 서버 측 check_admin_access
- [ ] settings 기본값 fallback

### 배포 시
- [ ] Railway: API smoke test 통과
- [ ] Vercel: 프론트 빌드 성공
- [ ] CORS: Vercel 도메인이 API 호출 가능
- [ ] 인증: 쿠키가 cross-origin에서 동작 (SameSite 설정 확인)
