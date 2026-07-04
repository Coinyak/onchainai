# Grok 작업 지시서: OnchainAI 전체 스펙 구현 (§A~§H)

## 프로젝트

OnchainAI — Rust 단일 바이너리 (Leptos SSR + Axum + rmcp + sqlx).
경로: `/Users/hoyeon/OnchainAI`
스펙 문서: `docs/AGENT_NATIVE_OPS_SPEC.md`

## 목표

프론트엔드 분리 전체 실행:
1. **§A**: 61개 Leptos server fn → Axum JSON API 핸들러 (`/api/v2/*`)
2. **§B**: Next.js + React + Tailwind 프론트엔드 구축 (34 컴포넌트, 20 페이지)
3. **§D**: 어드민 패널 확장 (콘텐츠 관리, 크롤러 스케줄)
4. **§E**: Vercel + Railway 이중 배포 구조, Dockerfile 단순화
5. **§F**: Phase 1~4 전체 마이그레이션 실행
6. **§H**: 검증 체크리스트 통과

**§C (MCP 개선)은 이미 구현 완료됨** (브랜치 `feat/mcp-agent-native-improvements`, 커밋 `0c85c1f`).

---

## 절대 규칙

1. **기존 백엔드 로직 재사용, 재발명 금지**: 모든 SQL 상수, 검증 함수, 권한 확인, 모델을 기존 코드에서 import. 새로 작성 금지.
2. **인증은 쿠키 기반 유지**: `session_from_parts(parts, pool, jwt_secret, issuer)` 호출. Bearer 토큰 새로 만들지 마라.
3. **`SUPABASE_SERVICE_KEY`/`JWT_SECRET` 클라이언트 노출 금지**: Next.js 클라이언트 코드에 secrets 없음.
4. **sqlx 파라미터화 쿼리만**: raw SQL string concatenation 금지.
5. **admin 검사는 서버 측**: 모든 admin 핸들러가 `require_admin` 호출.
6. **기존 Leptos 코드는 Phase 3까지 삭제 금지**: Phase 1~2는 병행. 기존 사이트가 계속 작동해야 함.
7. **매 단계 컴파일/테스트**: `cargo check --features ssr` + `cargo test --features ssr --lib` 통과 후 다음 단계.

---

## 현재 아키텍처

### 파일 구조

```
src/
├── lib.rs                    (644줄) — build_app() 라우터, AppState, CORS, rate limit
├── app.rs                    — Leptos App 컴포넌트 + shell
├── config.rs                 — Config 구조체 (siwx_domain, jwt_secret, ...)
├── auth/
│   ├── session.rs            (256줄) — SessionUser, cookie_value, optional_session_result
│   ├── session_ssr.rs        (745줄) — session_from_parts, JWT 발급/검증, load_session_user
│   ├── guard.rs              (80줄)  — require_admin, require_user, AuthError
│   ├── routes.rs             — GitHub OAuth, 로그아웃 라우트
│   ├── email.rs              — 매직링크
│   ├── siwx.rs               — SIWX 챌린지/검증
│   └── onboarding.rs         — 온보딩 완료/스킵
├── models/
│   ├── mod.rs                — SiteSettings, Source, SiwxSession, sanitize_site_settings_for_public
│   ├── tool.rs               (641줄) — Tool 구조체 (50+ 필드), sanitize_tool_for_public_response
│   ├── category.rs           — Category
│   ├── comment.rs            — Comment, Bookmark, Upvote
│   ├── featured.rs           — FeaturedCard
│   ├── review.rs             — OperatorVerdict, ReviewEntry, ReviewRun, ToolOfficialLink
│   ├── submission.rs         — ToolSubmission, ToolReport, ToolClaimRequest
│   └── user.rs               — Profile, ProfilePublic
├── server/
│   ├── mod.rs                — 모듈 선언
│   ├── queries.rs            (470줄) — public_tool_where!() 매크로, 모든 SQL 상수
│   ├── mcp.rs                (612줄) — MCP JSON-RPC 5 도구
│   ├── mcp_search.rs         — McpToolSummary, McpSearchPage, mcp_search_tools
│   ├── tool_categories.rs    — PUBLIC_TOOL_CATEGORY_IDS (14개), is_public_tool_category
│   ├── rate_limit.rs         — check_mcp_ip_rate_limit, check_user_rate_limit
│   ├── operator_harness.rs   — 이미 Axum 핸들러 (변환 불필요)
│   ├── review_persistence.rs — apply_operator_review_in_tx, list_official_links, ...
│   ├── functions.rs          (163줄) — request_context(), GetCurrentUser, CheckAdminAccess
│   └── functions/
│       ├── public_tools.rs   (1183줄) — 14개 server fn + Tool, ToolFilters, BrowserDataPayload 등
│       ├── comments_bookmarks.rs (310줄) — 7개 server fn + CommentView
│       ├── admin_review.rs   — 7개 server fn
│       ├── admin_users_comments.rs — 7개 server fn
│       ├── crawler_admin.rs  — 2개 server fn
│       ├── site_settings.rs  — 3개 server fn + UpdateSiteSettingsPayload
│       ├── submissions_workbench/
│       │   ├── submission_intake.rs — SubmitTool, ListMySubmissions
│       │   ├── workbench.rs   — GetToolTrustView, GetAdminWorkbenchSummary, GetAdminToolWorkbench, VerifyToolOfficialLink
│       │   └── reports_claims.rs — ReportTool, RequestToolClaim
│       └── taxonomy_featured/
│           ├── categories.rs  — ListAdminCategories, CreateCategory, UpdateCategory, DeleteCategory
│           └── featured.rs    — 7개 featured card server fn
├── components/               (34개 .rs 파일) — Leptos 컴포넌트
├── pages/                    (20개 .rs 파일) — Leptos 페이지
├── chains.rs                 — 체인 카탈로그
├── install_safety.rs         — 설치 위험 판정
└── style/output.css          (58571줄) — 수작업 CSS
```

### AppState

```rust
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Config,
    pub leptos_options: leptos::config::LeptosOptions,
}
```

### 인증 패턴 (모든 핸들러가 이 패턴을 따름)

기존 server fn:
```rust
let (parts, pool, config) = request_context()?;
// request_context()는 use_context::<Parts>(), use_context::<PgPool>(), use_context::<Config>()
```

새 Axum 핸들러 — `Parts`를 직접 조립해야 함:
```rust
async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<T>, ApiError> {
    // Parts 조립: headers에서 쿠키 추출, uri는 기본값
    let mut parts = axum::http::request::Parts::default();
    parts.headers = headers;
    let user = require_admin(&parts, &state.pool, &state.config).await?;
    // ... 동일한 쿼리/로직 ...
}
```

`session_from_parts`는 `parts.headers`에서 `cookie` 헤더를 읽어 `onchainai_access_token` 쿠키를 추출하므로, `parts.headers`에 쿠키가 있으면 충분.

### 이미 Axum 핸들러인 것들 (변환 불필요)

- `/mcp` — MCP JSON-RPC
- `/api/admin/operator/*` (5개)
- `/auth/*` (9개: github, github/switch, email, callback, logout, siwx/challenge, siwx/verify, onboarding/complete, onboarding/skip)

---

## Phase 1: 백엔드 API 전환 (§A)

### Step 0: 인프라

`src/server/api_v2/` 디렉토리 생성.

**`src/server/api_v2/mod.rs`**:
```rust
pub mod error;
pub mod auth;
pub mod public_tools;
pub mod user;
pub mod comments_bookmarks;
pub mod admin_review;
pub mod taxonomy_featured;
pub mod admin_users_comments;
pub mod crawler_admin;
pub mod site_settings;
pub mod submissions;
pub mod workbench;
pub mod reports_claims;

use axum::Router;
use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(public_tools::router(state.clone()))
        .merge(user::router(state.clone()))
        .merge(comments_bookmarks::router(state.clone()))
        .merge(admin_review::router(state.clone()))
        .merge(taxonomy_featured::router(state.clone()))
        .merge(admin_users_comments::router(state.clone()))
        .merge(crawler_admin::router(state.clone()))
        .merge(site_settings::router(state.clone()))
        .merge(submissions::router(state.clone()))
        .merge(workbench::router(state.clone()))
        .merge(reports_claims::router(state.clone()))
}
```

**`src/server/api_v2/error.rs`** — `ApiError` enum:
```rust
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    // 응답: { "error": { "code": "not_found", "message": "..." } }
    // NotFound -> 404, Unauthorized -> 401, Forbidden -> 403, BadRequest -> 400, Internal -> 500
}
```

**`src/server/api_v2/auth.rs`** — 인증 헬퍼:
```rust
use axum::http::request::Parts;
use axum::http::HeaderMap;
use crate::auth::guard::{require_admin, require_user, AuthError};
use crate::auth::session::{session_from_parts, optional_session_result, SessionUser};
use crate::AppState;
use super::error::ApiError;

/// HeaderMap에서 Parts 조립 (쿠키 포함).
pub fn parts_from_headers(headers: &HeaderMap) -> Parts {
    let mut parts = Parts::default();
    parts.headers = headers.clone();
    parts
}

pub async fn require_admin_from(state: &AppState, headers: &HeaderMap) -> Result<SessionUser, ApiError> {
    let parts = parts_from_headers(headers);
    require_admin(&parts, &state.pool, &state.config).await
        .map_err(|_| ApiError::Forbidden("not found".into()))
}

pub async fn require_user_from(state: &AppState, headers: &HeaderMap) -> Result<SessionUser, ApiError> {
    let parts = parts_from_headers(headers);
    require_user(&parts, &state.pool, &state.config.jwt_secret, &state.config.jwt_issuer())
        .await
        .map_err(|_| ApiError::Unauthorized("sign in required".into()))
}

pub async fn optional_user_from(state: &AppState, headers: &HeaderMap) -> Result<Option<SessionUser>, ApiError> {
    let parts = parts_from_headers(headers);
    let result = session_from_parts(&parts, &state.pool, &state.config.jwt_secret, &state.config.jwt_issuer()).await;
    Ok(optional_session_result(result).map_err(|e| ApiError::Internal(e.to_string()))?)
}
```

**`src/server/mod.rs`**에 `pub mod api_v2;` 추가.

**`src/lib.rs`** `build_app()`에 추가:
```rust
// 기존 app_routes 정의 후, merge 전에 추가
let api_v2_routes = crate::server::api_v2::router(state.clone());
// Router::new() 최종 merge에 .merge(api_v2_routes) 추가
```

**검증**: `cargo check --features ssr` 통과.

---

### Step 1: 공개 카탈로그 API (14개)

`src/server/api_v2/public_tools.rs` 생성. 기존 `src/server/functions/public_tools.rs`의 fetch 함수들을 import하여 재사용.

**import할 것들** (기존 코드에서):
- `fetch_categories`, `fetch_tool_by_slug`, `fetch_list_tools`, `fetch_count_tools`, `fetch_chain_counts`, `fetch_tool_comment_counts`
- `fetch_public_dashboard_snapshot`, `clamp_dashboard_list_limit`
- `validate_search_tools_input`, `validate_tool_list_request`, `validate_tool_filters`
- `clamp_list_tools_limit`, `clamp_browser_page_param`, `browser_visible_limit_for_page`
- `sanitize_tools_for_public_response`, `sanitize_tool_for_public_response`
- `ToolListRequest`, `ToolFilters`, `BrowserDataPayload`, `LoadBrowserDataRequest`
- `MyToolkitPayload`, `ToolkitToolView`, `UpdateToolkitItemPayload`, `ToolComparisonView`
- SQL: `RECENT_APPROVED_TOOLS_SQL`, `SEARCH_APPROVED_TOOLS_SQL`, `APPROVED_TOOL_BY_SLUG_SQL`, `COUNT_APPROVED_TOOLS_SQL`, `CHAIN_COUNTS_SQL`, `APPROVED_TOOLS_BY_SLUGS_SQL`, `USER_TOOLKIT_SQL`
- `ListMyToolkit`, `UpdateToolkitItem`, `CompareTools` — 기존 server fn 내부 로직을 fetch 함수로 추출하여 재사용. 이미 `fetch_` prefixed 함수들이 있으면 그것을 사용.

| Method | Path | 기존 server fn | 인증 | 입력 |
|--------|------|---------------|------|------|
| GET | `/api/v2/tools/recent` | GetRecentTools | 없음 | `?limit=` |
| GET | `/api/v2/categories` | GetCategories | 없음 | |
| GET | `/api/v2/tools/search` | SearchTools | 없음 | `?query=&function=&chain=` |
| GET | `/api/v2/tools/:slug` | GetToolBySlug | 없음 | |
| GET | `/api/v2/tools/count` | CountTools | 없음 | `?function=&chain=` (ToolFilters) |
| GET | `/api/v2/chains` | GetChainCounts | 없음 | `?limit=` |
| POST | `/api/v2/tools/list` | ListToolsV1 | 없음 | JSON body: `ToolListRequest` |
| POST | `/api/v2/browser-data` | LoadBrowserData | 없음 | JSON body: `LoadBrowserDataRequest` |
| GET | `/api/v2/dashboard` | GetPublicDashboardSnapshot | 없음 | `?limit=` |
| GET | `/api/v2/toolkit` | ListMyToolkit | user | |
| PUT | `/api/v2/toolkit/:slug` | UpdateToolkitItem | user | JSON body: `UpdateToolkitItemPayload` |
| GET | `/api/v2/tools/compare` | CompareTools | optional | `?slugs=slug1,slug2` |
| GET | `/api/v2/tools/comment-counts` | GetToolCommentCounts | 없음 | `?slugs=slug1,slug2` |
| GET | `/api/v2/tools/:slug/comment-count` | GetToolCommentCount | 없음 | |

**주의**:
- `ListMyToolkit`, `UpdateToolkitItem`, `CompareTools`는 기존 server fn 내부에서 직접 쿼리를 실행함. fetch 함수가 없으므로, server fn 본문의 로직을 핸들러로 복사하는 것이 아니라, server fn이 호출하는 내부 로직을 그대로 핸들러에서 실행. 필요한 SQL 상수와 모델은 import.
- `GetToolCommentCount`는 스펙에 누락되어 있지만 `comments_bookmarks.rs`에 존재. 추가.
- `CountTools`는 `ToolFilters`를 받으므로 query param으로 매핑하기 복잡할 수 있음. JSON body POST로 받거나 개별 query param으로 분해.

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib`

---

### Step 2: 인증 + 사용자 API (2개)

`src/server/api_v2/user.rs`:

| Method | Path | 기존 server fn | 인증 | 비고 |
|--------|------|---------------|------|------|
| GET | `/api/v2/me` | GetCurrentUser | optional | 쿠키에서 세션 |
| GET | `/api/v2/admin/check` | CheckAdminAccess | admin | |

`GetCurrentUser` 구현 참고: `request_context()` → `session_from_parts` → `optional_session_result` → `append_session_hint_if_missing`. 새 핸들러에서도 동일하게. 단, `append_session_hint_if_missing`은 Leptos `ResponseOptions`를 사용하므로 Axum에서는 직접 Set-Cookie 헤더를 추가해야 함.

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib`

---

### Step 3: 댓글/북마크 API (7개)

`src/server/api_v2/comments_bookmarks.rs`:

**import할 것들**:
- `CommentView`, `CommentRow`, `TOGGLE_UPVOTE_SQL`, `TOGGLE_BOOKMARK_SQL`
- `validate_comment_content`, `resolve_bookmark_tool_id`
- `APPROVED_TOOL_ID_BY_SLUG_SQL`, `TOOL_COMMENTS_NEW_SORT_SQL`, `TOOL_COMMENTS_TOP_SORT_SQL`, `TOOL_COMMENT_COUNT_BY_SLUG_SQL`, `IS_BOOKMARKED_SQL`
- `check_user_rate_limit`, `UserRateLimitAction`

| Method | Path | 기존 server fn | 인증 | 입력 |
|--------|------|---------------|------|------|
| GET | `/api/v2/tools/:slug/comments` | GetToolComments | optional | `?sort=new\|top` |
| GET | `/api/v2/tools/:slug/comment-count` | GetToolCommentCount | 없음 | |
| POST | `/api/v2/tools/:slug/comments` | CreateComment | user | JSON: `{ "content": "...", "parent_id": null }` |
| POST | `/api/v2/comments/:id/upvote` | ToggleUpvote | user | |
| GET | `/api/v2/tools/:slug/bookmark` | IsBookmarked | optional | |
| PUT | `/api/v2/tools/:slug/bookmark` | SetBookmark | user | JSON: `{ "starred": true }` |
| POST | `/api/v2/tools/:slug/bookmark` | ToggleBookmark | user | |

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib`

---

### Step 4: 어드민 도구 심사 API (7개)

`src/server/api_v2/admin_review.rs`:

**import할 것들**: `admin_review.rs`의 모든 fetch/validate 함수, `apply_operator_review_in_tx`, 관련 SQL.

| Method | Path | 기존 server fn | 인증 |
|--------|------|---------------|------|
| GET | `/api/v2/admin/pending` | ListPendingTools | admin |
| GET | `/api/v2/admin/stats` | GetAdminDashboardStats | admin |
| GET | `/api/v2/admin/review-queue` | ListReviewQueue | admin |
| POST | `/api/v2/admin/review` | ReviewTool | admin |
| POST | `/api/v2/admin/approval` | SetToolApproval | admin |
| GET | `/api/v2/admin/referral-stats` | GetReferralDashboardStats | admin |
| PUT | `/api/v2/admin/tool-referral` | UpdateToolReferral | admin |

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib`

---

### Step 5: 어드민 분류/추천 API (11개)

`src/server/api_v2/taxonomy_featured.rs`:

| Method | Path | 기존 server fn | 인증 | 비고 |
|--------|------|---------------|------|------|
| GET | `/api/v2/featured` | GetFeaturedCards | 없음 | |
| GET | `/api/v2/admin/featured` | ListFeaturedCards | admin | |
| POST | `/api/v2/admin/featured` | CreateFeaturedCard | admin | multipart 가능 |
| PUT | `/api/v2/admin/featured/:id` | UpdateFeaturedCard | admin | |
| DELETE | `/api/v2/admin/featured/:id` | DeleteFeaturedCard | admin | |
| POST | `/api/v2/admin/featured/upload` | UploadFeaturedImage | admin | multipart |
| GET | `/api/v2/admin/featured/search` | SearchToolsForPicker | admin | `?query=` |
| GET | `/api/v2/admin/categories` | ListAdminCategories | admin | |
| POST | `/api/v2/admin/categories` | CreateCategory | admin | |
| PUT | `/api/v2/admin/categories/:id` | UpdateCategory | admin | |
| DELETE | `/api/v2/admin/categories/:id` | DeleteCategory | admin | |

**multipart**: `UploadFeaturedImage`와 `CreateFeaturedCard`는 Axum `Multipart` 추출기 사용. 기존 로직 참고.

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib`

---

### Step 6: 어드민 사용자/댓글/크롤러/설정 API (12개)

`src/server/api_v2/admin_users_comments.rs` + `src/server/api_v2/crawler_admin.rs` + `src/server/api_v2/site_settings.rs`:

**import할 것들 (site_settings)**: `UpdateSiteSettingsPayload`, `parse_search_keywords`, `validate_update_site_settings_input`, `SiteSettingsValidationInput`, `sanitize_site_settings_for_public`

| Method | Path | 기존 server fn | 인증 |
|--------|------|---------------|------|
| GET | `/api/v2/admin/users` | ListAdminUsers | admin |
| PUT | `/api/v2/admin/users/:id/ban` | SetUserBanned | admin |
| PUT | `/api/v2/admin/users/:id/admin` | SetUserAdmin | admin |
| DELETE | `/api/v2/admin/users/:id` | DeleteUser | admin |
| GET | `/api/v2/admin/comments` | ListAdminComments | admin |
| DELETE | `/api/v2/admin/comments/:id` | DeleteAdminComment | admin |
| DELETE | `/api/v2/admin/comments/:id/ban-author` | DeleteCommentAndBanUser | admin |
| GET | `/api/v2/admin/crawler/sources` | ListCrawlerSources | admin |
| POST | `/api/v2/admin/crawler/trigger` | TriggerCrawlerSource | admin |
| GET | `/api/v2/settings` | GetSiteSettings | 없음 |
| GET | `/api/v2/admin/settings` | GetAdminSiteSettings | admin |
| PUT | `/api/v2/admin/settings` | UpdateSiteSettings | admin |

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib`

---

### Step 7: 제출/워크벤치/신고/클레임 API (8개)

`src/server/api_v2/submissions.rs` + `src/server/api_v2/workbench.rs` + `src/server/api_v2/reports_claims.rs`:

| Method | Path | 기존 server fn | 인증 | 입력 |
|--------|------|---------------|------|------|
| POST | `/api/v2/submit` | SubmitTool | user | JSON: `ToolSubmissionPayload` |
| GET | `/api/v2/my-submissions` | ListMySubmissions | user | |
| GET | `/api/v2/admin/trust/:slug` | GetToolTrustView | admin | |
| GET | `/api/v2/admin/workbench/summary` | GetAdminWorkbenchSummary | admin | |
| GET | `/api/v2/admin/workbench/:slug` | GetAdminToolWorkbench | admin | |
| POST | `/api/v2/admin/verify-link` | VerifyToolOfficialLink | admin | JSON |
| POST | `/api/v2/tools/:slug/report` | ReportTool | user | JSON: `{ "reason": "..." }` |
| POST | `/api/v2/tools/:slug/claim` | RequestToolClaim | user | JSON: `ToolClaimRequest` |

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib` + `cargo clippy --features ssr -- -W clippy::all` + `cargo fmt --check`

---

## Phase 2: 프론트엔드 구축 (§B)

### Step 8: Next.js 프로젝트 초기화

`/Users/hoyeon/OnchainAI/frontend/` 디렉토리 생성.

```bash
npx create-next-app@latest frontend --typescript --tailwind --app --no-src-dir --import-alias "@/*"
```

기술 스택:
- Next.js 15 (App Router, SSR)
- React 19
- Tailwind CSS v4
- TypeScript
- TanStack Query (서버 상태)
- `credentials: 'include'` 로 API 호출

### Step 9: 디자인 토큰 → Tailwind config

`frontend/tailwind.config.ts`:

DESIGN.md의 모든 토큰을 매핑:
- colors: primary, secondary, tertiary, neutral-bg, neutral-surface, neutral-hover, border, border-strong, text-muted, error, success, on-tertiary
- borderRadius: sm(6px), md(8px), lg(12px), full(9999px)
- spacing: xs(4), sm(8), md(16), lg(24), xl(32), 2xl(48), gutter(16), margin(24)
- fontSize: h1(28px/700), h2(20px/600), h3(16px/600), body-md(14px/400), body-sm(13px/400), label-caps(11px/600), code(13px/400)
- fontFamily: Inter (sans), JetBrains Mono (mono)

DESIGN.md의 모든 규칙 준수:
- light mode only
- orange(#E76F00)는 primary CTA, focus ring, active filter dot, links만
- gradient 금지, dark mode 금지, emoji 금지
- shadow 최소화 (1px border + tonal layering)
- 모바일 body text 16px 이상, touch target 44x44px

### Step 10: API 클라이언트 + 인증

`frontend/lib/api.ts`:
```typescript
const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, {
    ...options,
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', ...options?.headers },
  });
  if (!res.ok) {
    const error = await res.json().catch(() => ({ error: { message: 'Request failed' } }));
    throw new Error(error.error?.message || 'Request failed');
  }
  return res.json();
}
```

`frontend/lib/auth.tsx` — React Context로 인증 상태 관리. `/api/v2/me` 호출로 현재 사용자 확인. 쿠키는 HttpOnly이므로 클라이언트에서 읽지 못함. 서버 컴포넌트에서 `cookies()`로 전달.

### Step 11: P0 페이지 (4개)

| 페이지 | Next.js 라우트 | 기존 Leptos | 주요 컴포넌트 |
|--------|---------------|------------|-------------|
| 홈 | `/` | `frontend/app/page.tsx` | FeaturedCarousel, SearchBar, PromoCards (CategoryGrid removed §11-4) |
| 도구 브라우저 | `/tools` | `pages/tools_list.rs` + `components/tools_browser.rs` | ToolList, Sidebar, FilterPanel, Pagination, PreviewPanel |
| 도구 상세 | `/tools/[slug]` | `pages/tool_detail.rs` + `components/tool_detail_content.rs` | ToolDetail, InstallSection, TrustFacts, CommentSection |
| 로그인 | `/login` | `pages/login.rs` | LoginForm |

**거대 컴포넌트 분해 (200줄 이하)**:
- `tools_browser.rs` (1,400줄/50KB) → `ToolsBrowser` + `ToolList` + `FilterPanel` + `Pagination` + `PreviewPanel` + `BottomSheet`
- `sidebar.rs` (800줄/28KB) → `Sidebar` + `FilterChip` + `CategoryList` + `ChainList`
- `tool_detail_content.rs` (800줄/27KB) → `ToolDetail` + `InstallSection` + `TrustFacts` + `CommentSection` + `OfficialLinks`

각 컴포넌트는 기존 Leptos 컴포넌트의 UI/UX를 그대로 복제. `data-testid` 보존. DESIGN.md 디자인 시스템 준수.

**API 연결**: TanStack Query로 `/api/v2/*` 호출.

### Step 12: P0 컴포넌트 (기존 34개 중 핵심)

| 기존 Leptos 컴포넌트 | React 컴포넌트 | 크기 |
|---------------------|---------------|------|
| `tool_card.rs` (22KB) | `ToolCard` | |
| `search_bar.rs` (5.7KB) | `SearchBar` | |
| `sidebar.rs` (28KB) | `Sidebar` + `FilterChip` + `CategoryList` + `ChainList` | |
| `tool_detail_content.rs` (27KB) | `ToolDetail` + `InstallSection` + `TrustFacts` + `CommentSection` | |
| `bottom_sheet.rs` (4KB) | `BottomSheet` | |
| `top_nav.rs` (8KB) | `TopNav` | |
| `icons.rs` (5KB) | `Icons` (Lucide React) | |
| `copy_button.rs` (4KB) | `CopyButton` | |
| `highlighted_command.rs` (3.3KB) | `HighlightedCommand` | |
| `chain_strip.rs` (4.2KB) | `ChainStrip` | |
| `chain_logo.rs` (2.6KB) | `ChainLogo` | |
| `tool_logo.rs` (2.9KB) | `ToolLogo` | |
| `login_form.rs` (8.1KB) | `LoginForm` | |
| `login_modal.rs` (1.6KB) | `LoginModal` | |
| `skeleton.rs` (874B) | `Skeleton` | |
| `empty_state.rs` (2.5KB) | `EmptyState` | |
| `error_state.rs` (520B) | `ErrorState` | |
| ~~`category_grid.rs`~~ | ~~`CategoryGrid`~~ | Removed (§11-4); function discovery via sidebar |
| `featured_carousel.rs` (10.3KB) | `FeaturedCarousel` | |
| `promo_cards.rs` (1.6KB) | `PromoCards` | |

**아이콘**: DESIGN.md에 명시된 대로 Lucide SVG line icons 사용 (`lucide-react` 패키지). 색상 `#4B4B4B`.

### Step 13: P1 페이지 (12개)

| 페이지 | 라우트 | 기존 Leptos | 비고 |
|--------|--------|------------|------|
| 카테고리 | `/categories/[id]` | `pages/category.rs` | |
| 제출 | `/submit` | `pages/submit.rs` (24KB) | |
| 온보딩 | `/onboarding/profile` | `pages/onboarding.rs` | |
| 툴킷 | `/toolkit` | `pages/toolkit.rs` (10.8KB) | |
| 비교 | `/compare` | `pages/compare.rs` | |
| 대시보드 | `/dashboard` | `pages/dashboard.rs` | |
| 어드민 대시보드 | `/admin` | `pages/admin/dashboard.rs` | |
| 어드민 도구 | `/admin/tools` | `pages/admin/tools.rs` (20.7KB) | |
| 어드민 추천 | `/admin/featured` | `pages/admin/featured.rs` (24.4KB) | |
| 어드민 카테고리 | `/admin/categories` | `pages/admin/categories.rs` (11.7KB) | |
| 어드민 설정 | `/admin/settings` | `pages/admin/settings.rs` (12.5KB) | |
| 어드민 사용자 | `/admin/users` | `pages/admin/users.rs` (6.8KB) | |
| 어드민 댓글 | `/admin/comments` | `pages/admin/comments.rs` (5.2KB) | |
| 어드민 크롤러 | `/admin/crawler` | `pages/admin/crawler.rs` (7.9KB) | |

### Step 14: P1 컴포넌트 (나머지 14개)

| 기존 Leptos | React | |
|------------|-------|---|
| `comments_section.rs` (13.4KB) | `CommentsSection` + `CommentItem` + `CommentForm` | |
| `tool_listing_actions.rs` (13.1KB) | `ToolListingActions` | |
| `tool_finder.rs` (7.5KB) | `ToolFinder` | |
| `tool_trust_facts.rs` (1KB) | `TrustFacts` | |
| `claim_status_timeline.rs` (2.5KB) | `ClaimStatusTimeline` | |
| `official_links_list.rs` (2.5KB) | `OfficialLinksList` | |
| `preview_panel.rs` (1.4KB) | `PreviewPanel` | |
| `admin_review_decision_panel.rs` (16.1KB) | `AdminReviewDecisionPanel` | |
| `admin_review_timeline.rs` (4.3KB) | `AdminReviewTimeline` | |
| `admin_shell.rs` (3.3KB) | `AdminShell` (layout) | |
| `admin_context.rs` (1.4KB) | `AdminContext` (provider) | |
| `site_shell.rs` (334B) | `SiteShell` (layout) | |

### Step 15: 어드민 패널 확장 (§D)

#### D1: 콘텐츠 관리

**마이그레이션** `migrations/025_content_management.sql`:
```sql
ALTER TABLE site_settings
  ADD COLUMN IF NOT EXISTS hero_title TEXT,
  ADD COLUMN IF NOT EXISTS hero_subtitle TEXT,
  ADD COLUMN IF NOT EXISTS about_content TEXT,
  ADD COLUMN IF NOT EXISTS footer_links JSONB DEFAULT '[]';
```

**API**: `/api/v2/admin/settings` PUT 핸들러에 위 필드 추가. `UpdateSiteSettingsPayload`에 필드 추가. `SiteSettings` 모델에 필드 추가.

**Next.js 페이지**: `/admin/content` — 헤더 로고, 푸터 링크, About 본문, 홈 히어로 카피, 빈 상태 메시지 편집 폼.

#### D2: 크롤러 스케줄 관리

**마이그레이션** `migrations/026_crawler_schedule.sql`:
```sql
ALTER TABLE crawler_sources
  ADD COLUMN IF NOT EXISTS schedule_minutes INT NOT NULL DEFAULT 360,
  ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT true;
```

**API**: `/api/v2/admin/crawler/sources` GET에 schedule/enabled 포함. `/api/v2/admin/crawler/sources/:id` PUT (새 핸들러)으로 스케줄/활성화 업데이트.

**Next.js 페이지**: `/admin/crawler`에 스케줄 간격 입력 + enabled 토글 추가.

**마이그레이션 후**: `sqlx prepare` 실행.

**검증**: `cargo test --features ssr` + `cargo clippy --features ssr` + `cargo fmt --check`

---

## Phase 3: 전환 (§F Phase 3)

### Step 16: DNS + 배포 전환

#### Vercel 배포 설정

`frontend/` 디렉토리를 Vercel에 연결. 환경 변수:
- `NEXT_PUBLIC_API_URL=https://www.onchain-ai.xyz` (또는 API 도메인)

CORS: `src/lib.rs`의 `CorsLayer`에 Vercel 도메인 추가. 이미 `SITE_ORIGIN`이 있으므로, Vercel 도메인을 환경 변수에 추가하거나 `CorsLayer`에 명시.

#### Railway Dockerfile 단순화 (§E3)

분리 후 Dockerfile에서 WASM 빌드 제거:

```dockerfile
FROM rust:1.90-slim AS chef
WORKDIR /app
ARG CARGO_CHEF_VERSION=0.1.68
RUN cargo install cargo-chef --version "${CARGO_CHEF_VERSION}" --locked

FROM chef AS planner
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev curl perl && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features ssr --recipe-path recipe.json
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/
RUN cargo build --release --features ssr

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/onchainai /app/onchainai
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/public /app/public
CMD ["/app/onchainai"]
```

변경: cargo-leptos, WASM 빌드 단계, `style/` COPY 제거. `cargo build --release --features ssr`만.

### Step 17: Leptos 코드 제거

**주의: 이 단계는 Next.js 프론트가 Vercel에 배포되고 API가 Railway에서 작동 확인 후 실행.**

1. `src/lib.rs` `build_app()`에서 Leptos 라우트 제거. `leptos_routes_with_context`, `file_and_error_handler_with_context`, `provide_leptos_context` 제거. `/api/v2/*`를 `/api/*`로 전환 (또는 그대로 v2 유지).
2. `#[server(...)]` 함수 61개 제거.
3. `src/components/`, `src/pages/` 디렉토리 제거.
4. `style/output.css` 제거.
5. `Cargo.toml`에서 `hydrate` feature 제거. cargo-leptos 의존성 제거. Leptos 관련 deps 제거 (leptos, leptos_axum, leptos_meta, leptos_router 등).
6. `src/app.rs` 제거.
7. Dockerfile에서 WASM/cargo-leptos 의존성 제거 (Step 16에서 이미 수행).

**검증**: `cargo check --features ssr` + `cargo test --features ssr --lib` 통과. 기존 Leptos 코드 제거 후에도 백엔드 로직이 정상 작동.

---

## Phase 4: 검증 (§H)

### Step 18: 전체 검증

**API (Rust)**:
- [ ] `cargo test --features ssr` 전부 통과
- [ ] `cargo clippy --features ssr -- -W clippy::all` warning 없음
- [ ] `cargo fmt --check` 통과
- [ ] MCP visibility 필터 테스트 통과 (기존 28개 MCP 테스트)
- [ ] 인증 미들웨어 동작: admin 핸들러에 비로그인 접근 → 403
- [ ] 인증 미들웨어 동작: user 핸들러에 비로그인 접근 → 401
- [ ] 모든 공개 핸들러에 `sanitize_tools_for_public_response` 적용
- [ ] CORS: Vercel 도메인에서 API 호출 가능
- [ ] 쿠키 cross-origin 동작 (SameSite 설정 확인)

**프론트엔드 (Next.js)**:
- [ ] `next lint && next build` 통과
- [ ] DESIGN.md 토큰 준수 (Tailwind config)
- [ ] 기존 `data-testid` 보존
- [ ] 모바일 반응형: 44px touch target, 16px body text
- [ ] light mode only, orange 규칙 준수
- [ ] Lucide 아이콘, no emoji, no gradient

**어드민 확장**:
- [ ] 마이그레이션 025, 026 적용 + `sqlx prepare`
- [ ] 서버 측 `require_admin` 모든 admin 핸들러에 호출
- [ ] settings 기본값 fallback (사이트 깨지지 않음)

**배포**:
- [ ] Railway: API smoke test (`curl https://api.onchain-ai.xyz/api/v2/categories`)
- [ ] Vercel: 프론트 빌드 성공
- [ ] DNS: `www.onchain-ai.xyz` → Vercel, API 경로 → Railway

---

## 핵심 참고 파일

| 파일 | 용도 |
|------|------|
| `docs/AGENT_NATIVE_OPS_SPEC.md` | 전체 스펙 — 이 작업의 명세 |
| `DESIGN.md` | 디자인 시스템 (색상, 타이포, 컴포넌트, 레이아웃 규칙) |
| `src/lib.rs` | `build_app()` 라우터, `AppState`, CORS, rate limit |
| `src/server/functions.rs` | `request_context()`, `GetCurrentUser`, `CheckAdminAccess` |
| `src/server/functions/public_tools.rs` | 14개 공개 server fn + 모든 payload 구조체 |
| `src/server/functions/comments_bookmarks.rs` | 7개 댓글/북마크 server fn + `CommentView` |
| `src/server/functions/admin_review.rs` | 7개 어드민 심사 server fn |
| `src/server/functions/admin_users_comments.rs` | 7개 어드민 사용자/댓글 server fn |
| `src/server/functions/taxonomy_featured/categories.rs` | 4개 카테고리 server fn |
| `src/server/functions/taxonomy_featured/featured.rs` | 7개 추천 카드 server fn |
| `src/server/functions/crawler_admin.rs` | 2개 크롤러 server fn |
| `src/server/functions/site_settings.rs` | 3개 설정 server fn + `UpdateSiteSettingsPayload` |
| `src/server/functions/submissions_workbench/` | 8개 제출/워크벤치/신고 server fn |
| `src/server/queries.rs` | `public_tool_where!()` 매크로, 모든 SQL 상수 |
| `src/server/tool_categories.rs` | `PUBLIC_TOOL_CATEGORY_IDS` (14개) |
| `src/auth/guard.rs` | `require_admin`, `require_user`, `AuthError` |
| `src/auth/session.rs` | `SessionUser`, `cookie_value`, `optional_session_result` |
| `src/auth/session_ssr.rs` | `session_from_parts`, JWT 발급/검증 |
| `src/models/mod.rs` | `SiteSettings`, `Source`, `sanitize_site_settings_for_public` |
| `src/models/tool.rs` | `Tool` (50+ 필드), `sanitize_tool_for_public_response` |
| `src/models/comment.rs` | `Comment`, `Bookmark`, `Upvote` |
| `src/models/category.rs` | `Category` |
| `src/models/featured.rs` | `FeaturedCard` |
| `src/models/review.rs` | `OperatorVerdict`, `ReviewEntry`, `ReviewRun`, `ToolOfficialLink` |
| `src/models/submission.rs` | `ToolSubmission`, `ToolReport`, `ToolClaimRequest` |
| `src/models/user.rs` | `Profile`, `ProfilePublic` |
| `src/config.rs` | `Config` (siwx_domain, jwt_secret, jwt_issuer) |
| `Dockerfile` | 현재 cargo-chef + Leptos + WASM 빌드 |
| `migrations/` | 024까지 존재. 025, 026 추가 필요 |

## 완료 조건

- [ ] 61개 server fn에 대응하는 Axum 핸들러가 `/api/v2/*`에 존재
- [ ] Next.js 프론트엔드가 Vercel에 배포됨
- [ ] Rust API가 Railway에 배포됨
- [ ] 기존 Leptos 코드 제거됨 (Phase 3)
- [ ] Dockerfile이 Leptos/WASM 없이 단순화됨
- [ ] 어드민 콘텐츠 관리 + 크롤러 스케줄 관리 추가됨 (§D)
- [ ] `cargo test --features ssr` 통과
- [ ] `cargo clippy --features ssr -- -W clippy::all` warning 없음
- [ ] `cargo fmt --check` 통과
- [ ] `next lint && next build` 통과
- [ ] DESIGN.md 토큰 준수
- [ ] CORS cross-origin 동작 확인
- [ ] 인증 쿠키 cross-origin 동작 확인
