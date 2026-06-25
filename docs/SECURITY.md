# OnchainAI 보안 설계

> 관련 문서: [[INDEX]] | [[MVP_DESIGN]] | [[UI_UX_DESIGN]] | [[../AGENTS.md]]
>
> 작성: 2026-06-25. OWASP, SIWE Security Considerations, WalletConnect v2 Security, Rust API Security, Supabase RLS Best Practices 기반.

---

## 0. 보안 원칙

1. **Defense in Depth** — 여러 보안 계층 (HTTP 헤더, RLS, 입력 검증, 인증)
2. **Least Privilege** — 최소 권한만 부여 (RLS, 지갑 권한, API 스코프)
3. **Never Trust Client** — 모든 검증은 서버사이드
4. **Fail Secure** — 실패 시 안전한 방향으로 (에러 메시지는 정보 누출 금지)
5. **익명성 보장** — 닉네임으로만 활동, 인증 수단(이메일/지갑) 노출 금지

---

## 1. 인증 보안

### 1.1 GitHub OAuth + Email 매직링크 (Supabase Auth)

Supabase Auth가 인증 플로우 관리. 서버는 Supabase JWT 검증만 담당.

**보안 조치**:
- **HTTPS 강제**: 모든 인증 트래픽 TLS (HSTS 헤더)
- **JWT 검증**: 서버에서 모든 요청의 JWT 검증 (exp, nbf, iss, aud, sub)
- **짧은 토큰 수명**: Access token 15분, Refresh token 7일
- **Leeway 0**: 클럭 스큐 허용 안 함 (jsonwebtoken `leeway = 0`)
- **에러 메시지 통일**: "Invalid credentials" — 계정 존재 여부 누출 금지 (OWASP)
- **Rate limiting**: 인증 엔드포인트 5회/분/IP (governor crate)
- **Account lockout**: 5회 실패 시 15분 잠금 (exponential: 1s → 2s → 4s → 8s → 15min)
- **user_metadata 신뢰 금지**: JWT `raw_user_meta_data`는 유저가 수정 가능 → 권한 확인에 사용 금지 (Supabase RLS 베스트 프랙티스)
- **app_metadata만 신뢰**: 서버사이드에서만 설정 가능

### 1.2 SIWX 지갑 인증 (CAIP-122 / EIP-4361)

**보안 조치 (SIWE Security Considerations 기반)**:

- **서버사이드 메시지 생성**: SIWX 서명 메시지는 서버에서 전체 생성, 프론트엔드는 서명만 (피싱 방지)
- **Nonce**: 충분한 엔트로피의 1회성 nonce (16바이트 랜덤, base64), 사용 후 즉시 폐기
- **Domain binding**: 메시지에 `www.onchain-ai.xyz` 도메인 포함 → 다른 사이트에서 서명 재사용 방지
- **Expiration**: 서명 유효기간 5분 (issuedAt ~ expirationTime), 24시간 세션
- **리플레이 공격 방지**: nonce는 siwx_sessions에 저장, 검증 후 used 표시
- **서명 검증**:
  - EOA: eip191 서명 검증 (ethers-core)
  - 스마트 지갑: EIP-1271 (eth_call isValidSignature) + EIP-6492 (counterfactual)
  - Solana: ed25519 서명 검증 (solana-sdk)
- **체인 검증**: 서명 시 사용한 chain_id가 서버가 선언한 chain과 일치하는지 확인
- **메시지 검증 필수 항목**: domain, nonce, expirationTime, issuedAt, uri, chainId

**서명 메시지 예시**:
```
www.onchain-ai.xyz wants you to sign in with your Ethereum account:
0x1234...abcd

Sign in to OnchainAI to comment, upvote, and bookmark tools

URI: https://www.onchain-ai.xyz/auth/siwx
Version: 1
Chain ID: 1
Nonce: a3b1c9f2e7d4...
Issued At: 2026-06-25T12:00:00Z
Expiration Time: 2026-06-25T12:05:00Z
```

### 1.3 세션 관리

- **JWT**: HS256 서명 (서버 비밀키), 15분 access token + 7일 refresh token
- **세션 저장**: HttpOnly, Secure, SameSite=Strict 쿠키 (XSS로 접근 차단)
- **세션 무효화**: 로그아웃 시 서버에서 토큰 jti 블랙리스트 등록
- **재인증**: 민감한 작업(닉네임 변경, 이메일 변경, x402 결제) 시 재인증 필요 (OWASP)
- **Risk-based**: 새 IP/기기에서 로그인 시 추가 검증 (나중용)

---

## 2. 지갑 연결 보안 (WalletConnect v2 + 직접 연결)

### 2.1 지갑 연결 방식

| 방식 | 지갑 | 연결 |
|---|---|---|
| Browser Extension | MetaMask, Coinbase Wallet, Phantom | window.ethereum / window.solana 직접 호출 |
| WalletConnect v2 | 모바일 지갑 (Trust, Rainbow 등) | QR 코드 / 딥링크 |
| Smart Wallet | Coinbase Smart Wallet | EIP-1271 서명 검증 |

> MVP에서는 Browser Extension 직접 연결 우선. WalletConnect v2는 나중용.

### 2.2 WalletConnect v2 보안 (나중용)

- **E2E 암호화**: Relay 서버는 암호화된 페이로드만 중계 (메시지 내용 안 보임)
- **Domain verification**: WalletConnect Verify API로 dApp 도메인 인증 (VALID/INVALID/UNKNOWN)
- **최소 권한**: 요청할 메서드만 승인 (personal_sign만, eth_sendTransaction 금지)
- **세션 만료**: 기본 7일, 자동 만료 + 수동 disconnect UI
- **URI 검증**: WalletConnect URI 구조 엄격 검증 (malformed 입력 거부)
- **피싱 방지**: 도메인 수동 확인 유도, 북마크 권장

### 2.3 서명 보안

**핵심: OnchainAI는 인증 서명만 요청, 트랜잭션 서명 절대 안 함 (x402 결제 제외)**

- **인증 서명**: SIWX 메시지 서명만 (personal_sign / eth_signTypedData)
- **x402 결제**: 별도 플로우, 명확한 결제 정보 표시 후 서명
- **blind signing 금지**: 원시 hex 데이터 서명 금지, 항상 디코딩된 메시지 표시
- **서명 전 확인**: 지갑에 표시되는 메시지에 도메인(www.onchain-ai.xyz) 명시 → 피싱 시 유저가 도메인 불일치 감지

---

## 3. 웹 애플리케이션 보안 (Rust / Axum)

### 3.1 입력 검증 (Input Validation)

```rust
// validator crate 사용
#[derive(Debug, Deserialize, Validate)]
pub struct CommentRequest {
    #[validate(length(min = 1, max = 2000))]
    pub content: String,
    pub tool_id: Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ProfileRequest {
    #[validate(length(min = 2, max = 20, message = "Nickname must be 2-20 characters"))]
    #[validate(regex(path = "NICKNAME_REGEX", message = "Nickname must be alphanumeric + -_"))]
    pub nickname: String,
    #[validate(length(max = 200))]
    pub bio: Option<String>,
}
```

- **모든 입력 검증**: validator crate derive 매크로
- **닉네임**: 2-20자, 영문/숫자/하이픈/언더바만 (`^[a-zA-Z0-9_-]+$`)
- **댓글**: 1-2000자
- **Bio**: 최대 200자
- **URL**: repo_url, homepage, docs_url — URL 형식 검증 + 프로토콜(http/https만)
- **HTML 이스케이프**: 사용자 입력 표시 시 `& < > " '` → HTML 엔티티 (XSS 방지)
- **경로 검증**: `..`, `\0` 포함 시 거부 (path traversal 방지)

### 3.2 SQL 인젝션 방지

- **sqlx 파라미터화 쿼리만 사용**: `query_as!` 매크로, `$1`, `$2` 바인딩
- **문자열 보간 금지**: `format!()`로 SQL 조합 절대 금지
- **QueryBuilder**: 동적 쿼리 시 `sqlx::QueryBuilder` + `push_bind()` 사용
- **Rust 타입 시스템**: 컴파일 타임에 SQL 인젝션 방어 (sqlx 매크로)

### 3.3 XSS 방지

- **Leptos SSR**: 기본적으로 모든 텍스트 이스케이프 (`{user_input}` → 안전)
- **CSP 헤더**: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; frame-ancestors 'none'`
- **X-XSS-Protection**: `0` (레거시 필터 비활성화, CSP로 대체)
- **사용자 입력 렌더링**: 댓글, bio, 닉네임 — Leptos 기본 이스케이핑에 의존
- **innerHTML 금지**: 사용자 입력을 HTML로 직접 삽입 금지

### 3.4 CSRF 방지

- **SameSite=Strict 쿠키**: 인증 쿠키에 SameSite 속성
- **Origin 헤더 검증**: POST/PUT/DELETE 요청 시 `Origin` 헤더가 `www.onchain-ai.xyz`인지 확인
- **CSRF 토큰**: Leptos server function 호출 시 CSRF 토큰 검증 (Leptos 내장)
- **axum_csrf crate**: 추가 CSRF 보호 레이어 (선택적)

### 3.5 HTTP 보안 헤더

```rust
// 모든 응답에 자동 적용 (middleware)
"X-Frame-Options": "DENY"                    // 클릭재킹 방지
"X-Content-Type-Options": "nosniff"          // MIME 스니핑 방지
"X-XSS-Protection": "0"                      // 레거시 필터 비활성화 (CSP 사용)
"Referrer-Policy": "strict-origin-when-cross-origin"
"Content-Security-Policy": "default-src 'self'; script-src 'self'; ..."
"Strict-Transport-Security": "max-age=31536000; includeSubDomains"  // HSTS
"Permissions-Policy": "accelerometer=(), camera=(), geolocation=(), ..."
```

### 3.6 CORS 설정

- **프로덕션**: `https://www.onchain-ai.xyz`만 허용
- **개발**: `http://localhost:3000` 허용
- **메서드**: GET, POST, PUT, DELETE만
- **헤더**: Authorization, Content-Type, Accept만
- **Credentials**: true (쿠키 인증)

### 3.7 Rate Limiting

```rust
// governor crate
일반 API:       60회/분/IP
인증 엔드포인트:  5회/분/IP
댓글 작성:       10회/분/유저
도구 등록:       3회/시간/유저
검색:           30회/분/IP
```

- **429 Too Many Requests**: 초과 시 `Retry-After` 헤더와 함께 반환
- **인증 엔드포인트**: 더 엄격 (5회/분) — 브루트포스 방지
- **IP 기반**: `ConnectInfo<SocketAddr>`로 IP 추출

### 3.8 안전한 에러 처리

- **클라이언트**: 제네릭 에러 메시지 ("Invalid credentials", "Resource not found")
- **서버 로그**: 실제 에러는 `tracing::error!`로 내부 로깅
- **스택 트레이스**: 프로덕션에서 클라이언트에 노출 금지
- **DB 에러**: "An error occurred processing your request" (상세 정보 노출 금지)
- **404 vs 403**: 존재 여부 누출 방지 — 권한 없는 리소스는 404 반환 (IDOR 방지)

---

## 4. 데이터베이스 보안 (Supabase RLS)

### 4.1 RLS 기본 원칙

> Supabase가 PostgREST로 DB를 직접 노출하므로 RLS는 필수. anon 키만 있으면 전체 데이터 접근 가능.

- **모든 테이블에 RLS 활성화**: 예외 없음
- **정책 없음 = 접근 불가**: RLS만 켜고 정책 없으면 안전한 기본값 (모두 차단)
- **`to authenticated`**: 모든 정책에 명시 (익명 사용자 정책 평류 방지)
- **`(select auth.uid())` 패턴**: 함수 호출 캐싱 (행마다 실행 방지 → 성능)
- **service_role 키**: 서버사이드 전용, 클라이언트 노출 금지, git 금지

### 4.2 테이블별 RLS 정책

#### tools (공개 읽기, 등록자 쓰기)

```sql
-- 누구나 읽기 가능
CREATE POLICY "Public read tools" ON tools FOR SELECT TO anon, authenticated USING (true);

-- 인증된 유저만 등록
CREATE POLICY "Authenticated insert tools" ON tools FOR INSERT TO authenticated WITH CHECK (true);

-- 등록자 또는 관리자만 수정
CREATE POLICY "Owner update tools" ON tools FOR UPDATE TO authenticated
  USING ((select auth.uid()) = submitted_by)
  WITH CHECK ((select auth.uid()) = submitted_by);
```

#### profiles (본인만 접근)

```sql
-- 본인 프로필만 읽기/쓰기
CREATE POLICY "Self read profile" ON profiles FOR SELECT TO authenticated
  USING ((select auth.uid()) = id);
CREATE POLICY "Self update profile" ON profiles FOR UPDATE TO authenticated
  USING ((select auth.uid()) = id)
  WITH CHECK ((select auth.uid()) = id);
CREATE POLICY "Self insert profile" ON profiles FOR INSERT TO authenticated
  WITH CHECK ((select auth.uid()) = id);
```

> 닉네임/avatar는 공개 조회용 뷰(profiles_public)로 분리:
> `CREATE VIEW profiles_public WITH (security_invoker = true) AS SELECT id, nickname, avatar_url, auth_method FROM profiles;`

#### comments (공개 읽기, 인증 쓰기)

```sql
-- 누구나 댓글 읽기
CREATE POLICY "Public read comments" ON comments FOR SELECT TO anon, authenticated USING (true);

-- 인증된 유저만 댓글 작성 (본인 user_id만)
CREATE POLICY "Auth insert comments" ON comments FOR INSERT TO authenticated
  WITH CHECK ((select auth.uid()) = user_id);

-- 작성자만 수정/삭제
CREATE POLICY "Author update comments" ON comments FOR UPDATE TO authenticated
  USING ((select auth.uid()) = user_id)
  WITH CHECK ((select auth.uid()) = user_id);
CREATE POLICY "Author delete comments" ON comments FOR DELETE TO authenticated
  USING ((select auth.uid()) = user_id);
```

#### upvotes (본인만)

```sql
CREATE POLICY "Self read upvotes" ON upvotes FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id);
CREATE POLICY "Self insert upvotes" ON upvotes FOR INSERT TO authenticated
  WITH CHECK ((select auth.uid()) = user_id);
CREATE POLICY "Self delete upvotes" ON upvotes FOR DELETE TO authenticated
  USING ((select auth.uid()) = user_id);
```

#### bookmarks (본인만)

```sql
CREATE POLICY "Self read bookmarks" ON bookmarks FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id);
CREATE POLICY "Self insert bookmarks" ON bookmarks FOR INSERT TO authenticated
  WITH CHECK ((select auth.uid()) = user_id);
CREATE POLICY "Self delete bookmarks" ON bookmarks FOR DELETE TO authenticated
  USING ((select auth.uid()) = user_id);
```

#### siwx_sessions (서버사이드 전용)

```sql
-- RLS 활성화 + 정책 없음 = 클라이언트 접근 완전 차단
-- 서버는 service_role 키로 접근
ALTER TABLE siwx_sessions ENABLE ROW LEVEL SECURITY;
-- 정책 없음 → 모든 클라이언트 접근 차단
```

#### site_settings (관리자만)

```sql
-- 누구나 읽기 가능 (사이트 이름, 슬로건 등 공개 정보)
CREATE POLICY "Public read settings" ON site_settings FOR SELECT TO anon, authenticated USING (true);

-- 관리자만 수정
CREATE POLICY "Admin update settings" ON site_settings FOR UPDATE TO authenticated
  USING (EXISTS (SELECT 1 FROM profiles WHERE id = (select auth.uid()) AND is_admin = true))
  WITH CHECK (EXISTS (SELECT 1 FROM profiles WHERE id = (select auth.uid()) AND is_admin = true));
```

#### profiles (본인 + 관리자)

```sql
-- 본인 프로필 읽기/쓰기 (위에 정의됨)

-- 관리자는 모든 유저 프로필 읽기 가능
CREATE POLICY "Admin read all profiles" ON profiles FOR SELECT TO authenticated
  USING (EXISTS (SELECT 1 FROM profiles p WHERE p.id = (select auth.uid()) AND p.is_admin = true));

-- 관리자는 is_admin, is_banned 필드 수정 가능
-- (일반 필드는 본인만, admin 필드는 관리자만 — 별도 함수로 제어)
```

> 관리자 권한 체크는 `is_admin = true` 기반. RLS 정책에서 `EXISTS (SELECT 1 FROM profiles WHERE id = auth.uid() AND is_admin = true)` 패턴 사용.

### 4.3 RLS 성능 최적화

- **인덱스**: 모든 정책에서 사용하는 컬럼에 인덱스 (`user_id`, `tool_id`, `comment_id`)
- **`(select auth.uid())`**: 모든 정책에서 사용 (행마다 호출 방지)
- **security_invoker 뷰**: PostgreSQL 15+ `WITH (security_invoker = true)`
- **명시적 필터**: 앱 쿼리에서도 `.eq('user_id', userId)` 추가 (옵티마이저 도움)

### 4.4 RLS 테스트 (pgTap)

```sql
-- 권한 없는 유저가 타인 댓글 수정 시도 → 실패 (0 rows affected)
select tests.authenticate_as('attacker');
update comments set content = 'HACKED' where id = 'target-id';
-- 검증: 변경되지 않음
```

---

## 5. 환경변수 및 시크릿 관리

### .env.example (커밋 가능)

```env
# Database
DATABASE_URL=postgresql://user:pass@db.xxx.supabase.co:5432/postgres

# Supabase Auth
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_ANON_KEY=xxx
SUPABASE_SERVICE_KEY=xxx          # 서버 전용, 절대 클라이언트 노출 금지

# GitHub OAuth
GITHUB_CLIENT_ID=xxx
GITHUB_CLIENT_SECRET=xxx          # 서버 전용

# SIWX
SIWX_DOMAIN=www.onchain-ai.xyz
SIWX_SESSION_TTL=86400
JWT_SECRET=xxx                    # 32바이트 랜덤, 서버 전용

# GitHub API (크롤러)
GITHUB_API_TOKEN=xxx              # 서버 전용, star 동기화용

# x402
X402_FACILITATOR_URL=https://x402.org/facilitator
X402_PAY_TO_ADDRESS=0x...
```

### 보안 규칙

- **.env**: `.gitignore`에 추가, 절대 커밋 금지
- **.env.example**: 더미값만, 커밋 가능
- **SUPABASE_SERVICE_KEY**: RLS 바이패스 키, 서버사이드 전용
- **JWT_SECRET**: 최소 32바이트 랜덤 문자열
- **GITHUB_CLIENT_SECRET**: OAuth 클라이언트 시크릿, 서버 전용
- **Railway 환경변수**: Railway 대시보드에서 설정 (코드에 하드코딩 금지)

---

## 6. API 보안

### 6.1 MCP 서버 보안

- **인증 필수**: MCP 엔드포인트는 API 키 또는 인증 토큰 필요 (MVP 이후)
- **Rate limiting**: MCP 호출 100회/분/클라이언트
- **입력 검증**: 검색어, 카테고리, 체인 파라미터 검증
- **SQL 인젝션**: rmcp 핸들러도 sqlx 파라미터화 쿼리 사용

### 6.2 크롤러 보안

- **GitHub API 토큰**: 환경변수, Rate limit 방지 (5000/h)
- **외부 HTTP 요청**: `reqwest` 타임아웃 설정 (30초)
- **크롤 데이터 검증**: 이름/설명 이스케이프, 악성 스크립트 포함 시 제거
- **User-Agent**: 크롤링 시 명시적 User-Agent 헤더

---

## 7. 보안 체크리스트 (프로덕션 출시 전)

### 인증
- [ ] JWT 검증: exp, nbf, iss, aud, sub 모두 검증
- [ ] Access token 15분, Refresh token 7일
- [ ] 쿠키: HttpOnly, Secure, SameSite=Strict
- [ ] 에러 메시지: 계정 존재 여부 누출 금지
- [ ] Rate limiting: 인증 5회/분/IP
- [ ] Account lockout: 5회 실패 시 15분

### SIWX
- [ ] 서버사이드 메시지 생성
- [ ] Nonce: 16바이트 랜덤, 1회성, 사용 후 폐기
- [ ] Domain binding: www.onchain-ai.xyz
- [ ] Expiration: 서명 5분, 세션 24시간
- [ ] 서명 검증: eip191 (EOA) + EIP-1271 (smart wallet)

### 웹 (Rust/Axum)
- [ ] 입력 검증: 모든 요청 validator crate
- [ ] SQL: 파라미터화 쿼리만 (sqlx 매크로)
- [ ] XSS: Leptos 이스케이핑 + CSP 헤더
- [ ] CSRF: SameSite=Strict + Origin 검증 + CSRF 토큰
- [ ] 보안 헤더: X-Frame-Options, X-Content-Type-Options, CSP, HSTS, Referrer-Policy
- [ ] CORS: www.onchain-ai.xyz만 허용
- [ ] Rate limiting: governor crate
- [ ] 에러: 제네릭 메시지, 내부 로깅

### DB (Supabase)
- [ ] RLS: 모든 테이블 활성화
- [ ] 정책: SELECT, INSERT, UPDATE, DELETE 각각 정의
- [ ] `(select auth.uid())` 패턴 적용
- [ ] 인덱스: user_id, tool_id, comment_id
- [ ] service_role 키: 서버 전용
- [ ] profiles_public 뷰: security_invoker = true

### 시크릿
- [ ] .env: .gitignore 추가
- [ ] .env.example: 더미값만
- [ ] Railway 환경변수로 설정
- [ ] JWT_SECRET: 32바이트 랜덤
- [ ] GitHub API 토큰: 환경변수

### 지갑
- [ ] 인증 서명만 (트랜잭션 서명 금지, x402 제외)
- [ ] 서명 메시지에 도메인 명시
- [ ] blind signing 금지
- [ ] 최소 권한: personal_sign만

### 관리자
- [ ] 첫 가입자 자동 is_admin = true (트리거)
- [ ] `/admin/*` 라우트 서버사이드 is_admin 체크
- [ ] 비관리자 접근 시 404 (존재 누출 방지)
- [ ] site_settings: 관리자만 수정 (RLS)
- [ ] profiles is_admin/is_banned: 관리자만 수정 (RLS + 서버 검증)
- [ ] 도구 승인/거절: 관리자만 (서버사이드 검증)
- [ ] 댓글 삭제: 작성자 본인 또는 관리자
- [ ] 관리자 권한 부여/제거: 기존 관리자만
