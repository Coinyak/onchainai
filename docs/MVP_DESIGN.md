# OnchainAI MVP 설계 (Rust)

> 관련 문서: [[INDEX]] | [[UI_UX_DESIGN]] | [[SECURITY]] | [[../DESIGN]] | [[../AGENTS.md]]
>
> 작성: 2026-06-25. 경쟁자 정찰 + 사용자 기획 대화 기반.
> 2026-06-25 업데이트: TypeScript/Next.js → Rust 전체 스택 변경.

---

## 0. MVP 정의

**한 줄**: "파편화된 크립토 툴(MCP/CLI/SDK/API)을 자동으로 발견·정규화·노출하고, 사람과 agent 모두가 검색할 수 있는 허브"

**MVP 구성 3개**:
1. **크롤러** — 데이터 소스에서 자동 발견
2. **웹사이트** — 사람이 검색·탐색
3. **MCP 서버** — agent가 검색

**MVP에 없는 것**: x402 과금, 검증 배지, 셀프등록 UI, 추천엔진, 별도 CLI, 브라우저 확장

---

## 1. 전체 아키텍처

```
┌─────────────────────────────────────────────────────────┐
│                    데이터 소스                            │
│  CryptoSkill · web3-mcp-hub · GitHub topics · npm       │
└──────────────────────────┬──────────────────────────────┘
                           │ 크롤 (tokio-cron-scheduler / 1h)
                           ▼
┌─────────────────────────────────────────────────────────┐
│                 크롤러 (Rust, tokio)                      │
│  reqwest + scraper → 정규화 → 중복제거 → upsert (sqlx)   │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                 Postgres (Supabase 호스팅)                │
│  tools 테이블 · categories 테이블 · sources 테이블        │
└──────────┬──────────────────────────────┬───────────────┘
           │                              │
           ▼                              ▼
┌─────────────────────┐      ┌────────────────────────────┐
│  웹사이트 (Leptos)    │      │   MCP 서버 (Axum + rmcp)    │
│  SSR · 검색·카테고리  │      │  agent가 도구 검색·설치명령  │
└─────────────────────┘      └────────────────────────────┘
```

### Rust 기술 스택

| 구성요소 | 크레이트 | 비고 |
|---|---|---|
| 웹사이트 (SSR) | `leptos` + `leptos_axum` | SSR + 서버펑션, SEO 친화 |
| API + MCP 서버 | `axum` | HTTP 라우팅, MCP 엔드포인트 |
| MCP 프로토콜 | `rmcp` (modelcontextprotocol/rust-sdk) | 공식 Rust MCP SDK |
| 크롤링 | `reqwest` + `scraper` + `tokio` | 비동기 병렬 HTTP |
| DB | `sqlx` (Postgres) | 컴파일타임 SQL 검증 |
| 스케줄링 | `tokio-cron-scheduler` | 크롤 주기 실행 |
| 배포 | Docker → **Railway** | `main` 브랜치, Dockerfile 빌드 (`railway.json`), 상시 실행, ~$5/월 |
- 검색: Postgres FTS (MVP는 테이블이 작으므로 충분, 나중에 Typesense로)

---

## 2. 데이터베이스 스키마

### tools 테이블

```sql
CREATE TABLE tools (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  
  -- 식별
  name TEXT NOT NULL,              -- "BOB Gateway CLI"
  slug TEXT NOT NULL UNIQUE,       -- "bob-gateway-cli"
  description TEXT,                -- 정규화된 설명
  
  -- 분류 (3축 + 타입)
  function TEXT NOT NULL,         -- 'bridge' | 'swap' | 'wallet' | 'payments' | 'lending' | 'staking' | 'trading' | 'nft' | 'data' | 'dev-tool' | 'identity' | 'governance' | 'social' | 'ai-agent'
  asset_class TEXT DEFAULT 'crypto', -- 'crypto' | 'rwa' | 'derivatives' | 'stablecoins'
  actor TEXT DEFAULT 'human',     -- 'human' | 'ai-agent'
  type TEXT NOT NULL,             -- 'mcp' | 'cli' | 'sdk' | 'api' | 'skill' | 'x402'
  
  -- 연결
  repo_url TEXT,                   -- github.com/bob-collective/bob
  homepage TEXT,                   -- gobob.xyz
  npm_package TEXT,                -- @gobob/gateway-cli (있으면)
  install_command TEXT,            -- "npx @gobob/gateway-cli" 또는 "claude mcp add ..."
  mcp_endpoint TEXT,               -- https://mcp.zapper.xyz (있으면)
  
  -- 체인 지원
  chains TEXT[],                   -- ['bitcoin', 'ethereum', 'base', ...]
  
  -- 신뢰
  status TEXT DEFAULT 'community', -- 'verified' | 'official' | 'community'
  official_team TEXT,              -- "BOB Collective" (공식 팀인 경우)
  trust_score INT DEFAULT 0,       -- 나중용
  
  -- 등록 승인 (관리자 패널용)
  approval_status TEXT DEFAULT 'approved', -- 'pending' | 'approved' | 'rejected' (크롤 도구는 자동 approved)
  submitted_by UUID REFERENCES auth.users(id), -- 등록한 유저 (크롤은 NULL)
  rejection_reason TEXT,           -- 거절 사유 (관리자 입력)
  
  -- 메타
  license TEXT,                    -- 'MIT' | 'Apache-2.0' | ...
  pricing TEXT DEFAULT 'free',     -- 'free' | 'x402' | 'paid' | 'freemium'
  x402_price TEXT,                 -- "$0.003/call" (있으면)
  stars INT DEFAULT 0,             -- GitHub stars (크롤)
  last_commit_at TIMESTAMPTZ,      -- 최신 커밋 (신선도)
  
  -- 출처
  source TEXT NOT NULL,            -- 'cryptoskill' | 'web3-mcp-hub' | 'github' | 'npm' | 'manual'
  source_url TEXT,                 -- 원본 URL
  
  -- 타임스탬프
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_tools_function ON tools(function);
CREATE INDEX idx_tools_asset_class ON tools(asset_class);
CREATE INDEX idx_tools_actor ON tools(actor);
CREATE INDEX idx_tools_type ON tools(type);
CREATE INDEX idx_tools_status ON tools(status);
CREATE INDEX idx_tools_approval ON tools(approval_status);
CREATE INDEX idx_tools_slug ON tools(slug);
CREATE INDEX idx_tools_chains ON tools USING GIN(chains);
CREATE INDEX idx_tools_search ON tools USING GIN(to_tsvector('english', name || ' ' || description));
```

### sources 테이블 (크롤 상태 추적)

```sql
CREATE TABLE sources (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,       -- 'cryptoskill' | 'github-topics' | ...
  url TEXT NOT NULL,
  last_crawled_at TIMESTAMPTZ,
  crawl_status TEXT DEFAULT 'pending', -- 'pending' | 'success' | 'error'
  items_found INT DEFAULT 0,
  error_message TEXT
);
```

### site_settings 테이블 (관리자 패널용)

```sql
CREATE TABLE site_settings (
  id INT PRIMARY KEY DEFAULT 1,    -- 단일 행 (singleton)
  site_name TEXT DEFAULT 'OnchainAI',
  slogan TEXT DEFAULT 'Crypto tools, unified.',
  description TEXT DEFAULT 'Discover, install, and share crypto MCP, CLI, SDK, API, x402, RWA, and AI agent tools — all in one place.',
  mcp_endpoint TEXT DEFAULT 'npx mcp-remote www.onchain-ai.xyz/mcp',
  search_keywords TEXT[] DEFAULT ARRAY['mcp-server', 'crypto-mcp', 'web3-mcp', 'blockchain-mcp'],
  allow_free_registration BOOLEAN DEFAULT true,
  require_tool_approval BOOLEAN DEFAULT true,  -- 신규 도구 승인 필요 여부
  allow_x402_registration BOOLEAN DEFAULT false,
  updated_at TIMESTAMPTZ DEFAULT now()
);

-- 단일 행만 허용
INSERT INTO site_settings (id) VALUES (1) ON CONFLICT DO NOTHING;
```

### categories 테이블 (기능 카테고리, 이모지 없음, 뉴트럴 색상)

```sql
CREATE TABLE categories (
  id TEXT PRIMARY KEY,             -- 'bridge'
  label TEXT NOT NULL,             -- 'Bridge & Cross-chain'
  icon TEXT NOT NULL,              -- 'git-branch' (Lucide 아이콘 이름)
  description TEXT NOT NULL,       -- '체간 이동·브릿지·래핑'
  sort_order INT NOT NULL
);

-- 시드 데이터 (14개 기능 카테고리, 색상 구분 없음)
INSERT INTO categories (id, label, icon, description, sort_order) VALUES
  ('bridge', 'Bridge & Cross-chain', 'git-branch', '체간 이동·브릿지·래핑', 1),
  ('swap', 'Swap & DEX', 'arrow-left-right', '토큰 교환·유동성·라우팅', 2),
  ('wallet', 'Wallet & Custody', 'credit-card', '지갑 생성·관리·서명·MPC', 3),
  ('payments', 'Payments', 'dollar-sign', '결제·x402·송금·온램프', 4),
  ('lending', 'Lending & Borrowing', 'banknote', '대출·차입·청산', 5),
  ('staking', 'Staking & Yield', 'lock', '스테이킹·이자·수확', 6),
  ('trading', 'Trading & Perps', 'trending-up', '거래·영구선물·옵션·복사트레이딩', 7),
  ('nft', 'NFT & Marketplace', 'image', 'NFT 조회·발행·거래', 8),
  ('data', 'Data & Analytics', 'bar-chart', '시장 데이터·분석·인덱싱·오라클', 9),
  ('dev-tool', 'Developer Tools', 'terminal', 'RPC·인덱서·컨트랙트·디버그', 10),
  ('identity', 'Identity & KYA', 'fingerprint', '온체인 신원·어테스테이션·에이전트 인증', 11),
  ('governance', 'Governance & DAO', 'vote', '투표·제안·트레저리', 12),
  ('social', 'Social & Content', 'message-circle', '탈중앙 소셜·콘텐츠·크리에이터', 13),
  ('ai-agent', 'AI Agent', 'bot', '자율 에이전트·에이전트 경제·DeFAI', 14);
```

---

## 3. 크롤러 설계

### 데이터 소스별 전략

| 소스 | 방식 | 주기 | 예상 아이템 |
|---|---|---|---|
| **CryptoSkill** | GitHub 저장소 JSON 또는 웹사이트 스크랩 | 6h | ~1,342 skills + 121 MCP |
| **web3-mcp-hub** | registry.json 직접 fetch | 12h | ~50 MCP |
| **GitHub topics** | `mcp-server`, `crypto-mcp`, `web3-mcp`, `blockchain-mcp` 검색 | 1h | 발견 속도 핵심 |
| **npm** | `mcp`, `crypto`, `web3`, `blockchain` 키워드 신규 패키지 | 1h | BOB Gateway CLI 같은 것 선제 |

### 크롤 파이프라인 (Rust)

```
src/crawler/
├── mod.rs               # 오케스트레이터 (tokio::spawn 병렬 실행)
├── sources/
│   ├── mod.rs
│   ├── cryptoskill.rs   # CryptoSkill 스크랩 (reqwest + scraper)
│   ├── web3mcp.rs       # registry.json fetch (reqwest + serde_json)
│   ├── github.rs        # GitHub topics 검색 (GitHub API + reqwest)
│   └── npm.rs           # npm 신규 패키지 (npm API + reqwest)
├── normalizer.rs        # 소스별 데이터 → Tool 구조체로 정규화
├── deduper.rs           # repo_url 기준 중복 제거
└── scheduler.rs         # tokio-cron-scheduler 주기 실행
```

### 핵심 구조체

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Tool {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub function: String,           // 'bridge' | 'swap' | 'wallet' | ...
    pub asset_class: String,        // 'crypto' | 'rwa' | 'derivatives' | 'stablecoins'
    pub actor: String,              // 'human' | 'ai-agent'
    pub tool_type: String,          // 'mcp' | 'cli' | 'sdk' | 'api' | 'skill' | 'x402'
    pub repo_url: Option<String>,
    pub homepage: Option<String>,
    pub npm_package: Option<String>,
    pub install_command: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub chains: Vec<String>,
    pub status: String,             // 'verified' | 'official' | 'community'
    pub official_team: Option<String>,
    pub license: Option<String>,
    pub pricing: String,
    pub x402_price: Option<String>,
    pub stars: i32,
    pub last_commit_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source: String,
    pub source_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 각 소스 크롤러가 구현하는 트레이트
#[async_trait::async_trait]
pub trait SourceCrawler {
    async fn crawl(&self) -> Result<Vec<RawTool>>;
    fn source_name(&self) -> &str;
    fn interval(&self) -> cron::Schedule;
}
```

### 정규화 규칙

```
입력: 소스별 다른 형태 (CryptoSkill JSON, GitHub API JSON, npm JSON)
출력: Tool 구조체

매핑 예시:
  CryptoSkill skill →
    name: skill.name
    tool_type: 'skill'
    function: 매핑 (CryptoSkill 13카테고리 → OnchainAI 14개 기능)
    asset_class: 'crypto' (기본, README에서 RWA/derivatives 키워드 시 변경)
    actor: 'human' (기본, 'agent' 키워드 시 'ai-agent')
    source: 'cryptoskill'
    source_url: skill.github_url
    install_command: 'clawhub install ' + skill.slug

  GitHub repo →
    name: repo.name
    tool_type: README에서 추론 (mcp-server 있으면 'mcp', cli 있으면 'cli')
    function: README/topics에서 키워드 매칭
    asset_class: README에서 RWA/derivative/stablecoin 키워드 매칭
    actor: README에서 'agent'/'autonomous' 키워드 시 'ai-agent'
    stars: repo.stargazers_count
    last_commit_at: repo.pushed_at
    source: 'github'

  npm package →
    name: package.name
    tool_type: 'cli' (package.json의 bin 필드 있으면) 또는 'sdk'
    install_command: 'npx ' + package.name
    source: 'npm'
```

### 3축 자동 분류 (키워드 매칭)

```rust
/// 기능 분류
fn classify_function(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let rules: &[(&str, &[&str])] = &[
        ("bridge", &["bridge", "cross-chain", "gateway", "bob gateway", "wormhole", "layerzero"]),
        ("swap", &["swap", "dex", "uniswap", "jupiter", "1inch", "liquidity pool"]),
        ("wallet", &["wallet", "custody", "key", "sign", "mpc", "safe"]),
        ("payments", &["payment", "x402", "usdc", "invoice", "checkout", "onramp", "offramp"]),
        ("lending", &["lending", "borrow", "loan", "aave", "compound", "liquidation"]),
        ("staking", &["staking", "stake", "yield", "restake", "eigenlayer", "marinade"]),
        ("trading", &["trade", "trading", "perp", "perpetual", "futures", "options", "hyperliquid", "gmx", "dydx"]),
        ("nft", &["nft", "mint", "opensea", "collection", "magic eden"]),
        ("data", &["analytics", "price", "market data", "coingecko", "defillama", "indexer", "oracle", "subgraph"]),
        ("dev-tool", &["rpc", "sdk", "hardhat", "foundry", "compiler", "debug", "remix"]),
        ("identity", &["identity", "ens", "attestation", "worldcoin", "kya", "world id"]),
        ("governance", &["governance", "dao", "vote", "proposal", "treasury", "snapshot"]),
        ("social", &["social", "lens", "farcaster", "content", "creator", "mirror"]),
        ("ai-agent", &["agent", "autonomous", "ai agent", "eliza", "virtuals", "ai16z", "defai", "tiny.place"]),
    ];
    for (cat, keywords) in rules {
        if keywords.iter().any(|k| lower.contains(k)) {
            return cat;
        }
    }
    "dev-tool" // 기본
}

/// 자산 유형 분류
fn classify_asset_class(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if ["rwa", "real world asset", "treasury", "t-bill", "stock token", "ondo", "securitize"].iter().any(|k| lower.contains(k)) {
        "rwa"
    } else if ["derivative", "perpetual", "perp", "option", "futures", "synthetic"].iter().any(|k| lower.contains(k)) {
        "derivatives"
    } else if ["stablecoin", "usdc", "usdt", "dai", "stable"].iter().any(|k| lower.contains(k)) {
        "stablecoins"
    } else {
        "crypto"
    }
}

/// 주체 분류
fn classify_actor(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if ["agent", "autonomous", "ai agent", "agentic", "bot", "eliza", "tiny.place"].iter().any(|k| lower.contains(k)) {
        "ai-agent"
    } else {
        "human"
    }
}
```

### 크롤 스케줄

```rust
use tokio_cron_scheduler::{Job, JobScheduler};

// npm: 1시간마다
let npm_job = Job::new_async("0 0 * * * *", |_uuid, _l| {
    Box::pin(async { npm::crawl(&pool).await; })
})?;

// CryptoSkill: 6시간마다
let cs_job = Job::new_async("0 0 */6 * * *", |_uuid, _l| {
    Box::pin(async { cryptoskill::crawl(&pool).await; })
})?;

// web3-mcp-hub: 12시간마다
let w3_job = Job::new_async("0 0 */12 * * *", |_uuid, _l| {
    Box::pin(async { web3mcp::crawl(&pool).await; })
})?;

// GitHub topics: 1시간마다 (30분 오프셋)
let gh_job = Job::new_async("0 30 * * * *", |_uuid, _l| {
    Box::pin(async { github::crawl(&pool).await; })
})?;

scheduler.add(npm_job).await?;
scheduler.add(cs_job).await?;
scheduler.add(w3_job).await?;
scheduler.add(gh_job).await?;

// GitHub stars 동기화: 30분마다 기존 도구들의 star 수 갱신
// (신규 발견이 아닌 기존 repo_url의 stargazers_count 업데이트)
let star_sync_job = Job::new_async("0 */30 * * * *", |_uuid, _l| {
    Box::pin(async { github::sync_stars(&pool).await; })
})?;
scheduler.add(star_sync_job).await?;

// OnchainAI 자체 리포 self-listing: 시작 시 1회 자동 등록
// (source: 'self', status: 'official', OnchainAI 자체 GitHub repo)
onchainai::self_register(&pool).await?;

scheduler.start().await?;
```

### GitHub star 동기화

기존 tools 테이블의 모든 `repo_url`이 있는 도구에 대해 GitHub API `/repos/{owner}/{repo}` 호출하여 `stargazers_count` 갱신.

```rust
// src/crawler/sources/github.rs

/// 30분마다 실행: 기존 도구들의 GitHub star 수 동기화
pub async fn sync_stars(pool: &PgPool) {
    // repo_url이 있는 도구 최대 100개씩 배치 처리
    let tools = sqlx::query_as::<_, Tool>(
        "SELECT * FROM tools WHERE repo_url IS NOT NULL ORDER BY updated_at ASC LIMIT 100"
    ).fetch_all(pool).await?;

    for tool in tools {
        if let Some(repo_url) = &tool.repo_url {
            // github.com/{owner}/{repo} 파싱
            let (owner, repo) = parse_github_url(repo_url);
            // GET https://api.github.com/repos/{owner}/{repo}
            let resp: GithubRepoResponse = client.get(...)
                .header("Authorization", format!("Bearer {}", github_token))
                .send().await?.json().await?;
            
            sqlx::query("UPDATE tools SET stars = $1, last_commit_at = $2, updated_at = now() WHERE id = $3")
                .bind(resp.stargazers_count)
                .bind(resp.pushed_at)
                .bind(tool.id)
                .execute(pool).await?;
        }
        // Rate limit 방지: 10ms 대기 (GitHub API 5000/h)
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// OnchainAI 자체 리포를 tools 테이블에 self-register
pub async fn self_register(pool: &PgPool) {
    sqlx::query(r#"
        INSERT INTO tools (name, slug, description, function, asset_class, actor, type, 
                          repo_url, homepage, status, official_team, source, license, stars)
        VALUES ($1, $2, $3, 'dev-tool', 'crypto', 'ai-agent', 'mcp',
                $4, $5, 'official', 'OnchainAI', 'self', 'MIT', 0)
        ON CONFLICT (slug) DO NOTHING
    "#)
        .bind("OnchainAI")
        .bind("onchainai")
        .bind("Crypto tool directory — discover, install, and share MCP, CLI, SDK, API, x402 tools for humans and agents.")
        .bind("https://github.com/love/onchainai")  // 실제 repo URL
        .bind("https://www.onchain-ai.xyz")
        .execute(pool).await?;
}
```

> **Self-listing 이유**: OnchainAI 자체도 크립토 툴 디렉토리의 한 항목. 
> 자기 자신을 등록함으로써 "dogfooding" + GitHub 별 노출 + 사용자가 별/코멘트 가능.
> source: 'self'로 구분, status: 'official' 자동 부여.

---

## 3.5. 인증 시스템 (3-way 통합)

### 개요

댓글, 업보트, 북마크, 도구 등록에 인증 필요. 탐색(검색/필터/상세)은 인증 없이 자유.

| 방식 | 구현 | 대상 | 비고 |
|---|---|---|---|
| GitHub OAuth | Supabase Auth GitHub provider | 개발자 | avatar + username 자동 |
| Email 매직링크 | Supabase Auth Email | 일반 유저 | 닉네임 첫 로그인 시 설정 |
| 지갑 (SIWX) | CAIP-122 Sign-In-With-X (x402 V2) | 크립토 유저/AI agent | EVM + Solana, 스마트지갑 지원 |

### SIWX 지갑 인증 플로우

```
1. 유저: [Connect Wallet] 클릭
2. 프론트엔드: 지갑 연결 (MetaMask/Coinbase Wallet/Phantom)
3. 서버: nonce 생성 → siwx_sessions에 저장 → 클라이언트에 전달
4. 프론트엔드: CAIP-122 메시지 생성 (domain, nonce, expirationTime)
5. 유저: 지갑으로 메시지 서명
6. 프론트엔드: 서명을 서버로 전송
7. 서버: 서명 검증 (eip191 for EOA, eip1271 for smart wallet)
   → wallet_address 추출
   → profiles 테이블에 upsert (auth_method='siwx')
   → 세션 토큰 발급 (Supabase JWT)
8. 이후 요청: JWT로 인증
```

### x402 등록 연동

지갑 로그인 유저가 x402 도구 등록 시:
- 이미 지갑 연결됨 → 별도 연결 단계 없음
- x402 결제 → 자동으로 인증 세션 생성 (SIWX settle hook)
- GitHub/Email 유저가 x402 등록 시: 별도 지갑 연결 팝업

### 환경변수 추가

```env
# Supabase Auth
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_ANON_KEY=xxx
SUPABASE_SERVICE_KEY=xxx

# GitHub OAuth (Supabase provider에서 설정)
GITHUB_CLIENT_ID=xxx
GITHUB_CLIENT_SECRET=xxx

# SIWX
SIWX_DOMAIN=www.onchain-ai.xyz
SIWX_SESSION_TTL=86400          # 24h (초)
```

### Cargo.toml 인증 의존성 추가

```toml
# 인증
supabase-auth = "0.3"           # Supabase Auth 클라이언트
jsonwebtoken = "9"              # JWT 검증
ethers-core = "2"               # EVM 서명 검증 (eip191/eip1271)
solana-sdk = "2"                # Solana 서명 검증 (ed25519)
```

---

## 3.6. GitHub 리포 전략

### 리포 구성

```
onchainai/                     ← GitHub 리포 (github.com/love/onchainai)
├── README.md                  ← 프로젝트 소개 (별 받기 핵심)
├── LICENSE                    ← MIT
├── docs/                      ← 설계 문서
│   ├── MVP_DESIGN.md
│   ├── UI_UX_DESIGN.md
│   └── DESIGN.md              ← Stitch spec 디자인 토큰
├── src/                       ← Rust 소스
├── migrations/                ← DB 마이그레이션
├── Dockerfile
├── .env.example
└── style/
```

### README.md 전략 (별 유도)

```markdown
# OnchainAI

> Crypto tools, unified. Discover, install, and share MCP, CLI, SDK, API, x402, 
> RWA, and AI agent tools — all in one place.

[![GitHub stars](https://img.shields.io/github/stars/love/onchainai)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)]()

## Why?

Crypto tooling is fragmented across CryptoSkill, Smithery, npm, GitHub topics, 
and dozens of separate registries. **BOB Gateway CLI was released 2 days ago 
with AI agent docs — but no directory had it.** OnchainAI fixes this.

## Features

- Auto-discovery crawler (CryptoSkill, GitHub topics, npm, web3-mcp-hub)
- 3-axis classification (Function × Asset Class × Actor)
- MCP server for agents to search tools programmatically
- x402 payment integration for paid tools
- GitHub star sync (freshness indicator)

## Stack

Rust (Leptos SSR + Axum + rmcp + sqlx + tokio-cron-scheduler)

## Quick Start

```bash
git clone https://github.com/love/onchainai
cd onchainai
cp .env.example .env
docker build -t onchainai .
docker run -p 3000:3000 onchainai
```

## Design Docs

- [MVP Design](docs/MVP_DESIGN.md)
- [UI/UX Design](docs/UI_UX_DESIGN.md)
- [DESIGN.md](DESIGN.md) (Stitch spec for AI UI generation)
```

### 웹사이트 ↔ GitHub 별 연동

```
웹사이트 카드:                   GitHub 리포:
┌────┐ OnchainAI              github.com/love/onchainai
│logo│ [Official] [MCP]       ★ 342 stars (30분마다 동기화)
└────┘ ★ 342  comments 5      
      github.com/love/onchainai [↗]
```

- 크롤러가 30분마다 GitHub API로 star 수 갱신 (3.5절 sync_stars)
- 웹사이트 카드에 실시간 별 표시
- "GitHub ↗" 링크 → 리포로 이동 → 유저가 별 가능
- OnchainAI 자체도 self-listing으로 카드 표시 (dogfooding)

---

## 4. MCP 서버 설계 (Axum + rmcp)

### 개요

사이트 자체가 MCP 서버. agent가 연결하면 "크립토 툴 검색" 도구를 사용 가능.

### 엔드포인트

```
POST https://www.onchain-ai.xyz/mcp     → MCP Streamable HTTP (rmcp)
```

### 노출 도구 (MCP tools)

```
1. search_tools
   입력: { query: string, category?: string, type?: string, chain?: string }
   출력: [{ name, description, category, type, chains, install_command, verified, mcp_endpoint }]
   설명: "크립토 MCP/CLI/SDK/API 도구 검색"

2. get_tool_detail
   입력: { slug: string }
   출력: { name, description, category, type, chains, install_command, mcp_endpoint, repo_url, homepage, license, pricing, x402_price, stars, last_commit_at, verified }
   설명: "특정 도구 상세 정보"

3. list_categories
   입력: {}
   출력: [{ id, label, emoji, description, tool_count }]
   설명: "사용 가능한 카테고리 목록"

4. get_install_guide
   입력: { slug: string, platform: 'claude' | 'cursor' | 'generic' }
   출력: { command: string, config_json?: string, steps: string[] }
   설명: "특정 플랫폼용 설치 가이드"
```

### 구현 (rmcp + axum)

```rust
use rmcp::ServerHandler;
use rmcp::model::{ServerInfo, ServerCapabilities, Tool, CallToolResult};
use rmcp::service::RequestContext;

#[derive(Clone)]
pub struct OnchainAiMcp {
    pub pool: PgPool,
}

#[rmcp::tool(tool_box)]
impl OnchainAiMcp {
    #[tool(desc = "Search crypto MCP/CLI/SDK/API tools")]
    async fn search_tools(
        &self,
        #[tool(param)] query: String,
        #[tool(param)] category: Option<String>,
        #[tool(param)] chain: Option<String>,
    ) -> Result<CallToolResult, rmcp::Error> {
        let tools = search_tools_in_db(&self.pool, &query, category, chain).await?;
        Ok(CallToolResult::text(serde_json::to_string_pretty(&tools)?))
    }

    #[tool(desc = "Get detailed info for a specific tool by slug")]
    async fn get_tool_detail(
        &self,
        #[tool(param)] slug: String,
    ) -> Result<CallToolResult, rmcp::Error> {
        let tool = get_tool_by_slug(&self.pool, &slug).await?;
        Ok(CallToolResult::text(serde_json::to_string_pretty(&tool)?))
    }

    #[tool(desc = "List all available tool categories")]
    async fn list_categories(&self) -> Result<CallToolResult, rmcp::Error> {
        let cats = list_categories_db(&self.pool).await?;
        Ok(CallToolResult::text(serde_json::to_string_pretty(&cats)?))
    }

    #[tool(desc = "Get platform-specific install guide")]
    async fn get_install_guide(
        &self,
        #[tool(param)] slug: String,
        #[tool(param)] platform: String,
    ) -> Result<CallToolResult, rmcp::Error> {
        let guide = build_install_guide(&self.pool, &slug, &platform).await?;
        Ok(CallToolResult::text(serde_json::to_string_pretty(&guide)?))
    }
}

// Axum 라우터에 MCP 엔드포인트 마운트
let app = Router::new()
    .route("/mcp", post(handle_mcp_request))  // rmcp 핸들러
    .route("/api/*", ...);                    // 일반 API
```

### 설치 (사용자가 agent에 연결)

```json
{
  "mcpServers": {
    "onchainai": {
      "command": "npx",
      "args": ["mcp-remote", "https://www.onchain-ai.xyz/mcp"]
    }
  }
}
```

---

## 5. 웹사이트 설계 (Leptos SSR)

### 페이지 구조

```
/                     → 홈 (검색바 + 카테고리 그리드 + 최신 도구)
/tools                → 전체 도구 리스트 (필터: category, type, chain)
/tools/:slug          → 도구 상세 (설명, 설치, 체인, 신뢰, 예시)
/categories/:id       → 카테고리별 도구
/about                → 프로젝트 소개
```

### 홈 페이지

```
[로고: OnchainAI]
"크립토 툴을 하나로"

[검색바: "자산 추적, 거래, DeFi..."]

카테고리 그리드 (10개):
  📊 Asset Tracking    📈 Trading    🏗️ DeFi    🖼️ NFT
  📡 Data              👛 Wallet     🌉 Bridge  🔧 Dev Tools
  🪪 Identity          💳 Payments

최신 추가된 도구 (상위 8개):
  [카드] BOB Gateway CLI  · bridge · bitcoin,ethereum · 어제 추가
  [카드] Zapper MCP       · data · 60+ chains · verified
  ...
```

### 도구 카드

```
┌──────────────────────────────────┐
│ 🌉 BOB Gateway CLI        [verified] │
│ Bitcoin ↔ EVM 브릿지 CLI           │
│ 체인: BTC · ETH · Base · ...      │
│ 타입: CLI                          │
│ ⭐ 125 · 어제 업데이트              │
│ [설치 명령 복사]  [상세 보기]       │
└──────────────────────────────────┘
```

### 도구 상세 페이지

```
BOB Gateway CLI                    [verified] [official: BOB Collective]
────────────────────────────────────────────────────────────────
카테고리: 🌉 Bridge & Cross-chain
타입: CLI
체인: Bitcoin, Ethereum, Base, BNB, +8

설명:
BOB Gateway는 Bitcoin을 11+ 체인으로 1-클릭 이동시키는 CLI 도구.
AI agent용 문서 제공. 어제 v0.2.0 릴리즈 (send 명령 추가).

설치:
  npm install -g @gobob/gateway-cli
  [복사 버튼]

링크:
  [GitHub] [문서] [홈페이지]

신뢰:
  ✅ 공식 팀 (BOB Collective)
  ✅ GitHub 125 stars
  ✅ 어제 커밋 (활성)
  ⚠️ 아직 검증 배지 없음
```

### Leptos 컴포넌트 구조

```rust
// src/app.rs (개념)
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/" view=HomePage/>
                <Route path="/tools" view=ToolsListPage/>
                <Route path="/tools/:slug" view=ToolDetailPage/>
                <Route path="/categories/:id" view=CategoryPage/>
                <Route path="/about" view=AboutPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let tools = create_resource(|| (), |_| async { get_recent_tools(8).await });
    view! {
        <SearchBar/>
        // CategoryGrid removed 2026-06-26 — sidebar Function filter
        <Suspense fallback=|| "Loading...">
            <ToolList tools={tools}/>
        </Suspense>
    }
}

#[component]
fn ToolCard(tool: Tool) -> impl IntoView {
    view! {
        <div class="tool-card">
            <span class="emoji">{category_emoji(&tool.category)}</span>
            <h3>{tool.name.clone()}</h3>
            <p>{tool.description.clone()}</p>
            <div class="chains">{tool.chains.join(" · ")}</div>
            <button on:click=copy_install>{tool.install_command}</button>
        </div>
    }
}
```

### 서버펑션 (Leptos server functions → Axum)

```rust
#[server(GetRecentTools, "/api")]
pub async fn get_recent_tools(limit: i64) -> Result<Vec<Tool>, ServerFnError> {
    let pool = use_context::<PgPool>().unwrap();
    let tools = sqlx::query_as::<_, Tool>(
        "SELECT * FROM tools ORDER BY created_at DESC LIMIT $1"
    )
    .bind(limit)
    .fetch_all(&pool)
    .await?;
    Ok(tools)
}

#[server(SearchTools, "/api")]
pub async fn search_tools(query: String, category: Option<String>) -> Result<Vec<Tool>, ServerFnError> {
    let pool = use_context::<PgPool>().unwrap();
    // Postgres FTS 쿼리
    let tools = sqlx::query_as::<_, Tool>(
        "SELECT * FROM tools WHERE to_tsvector('english', name || ' ' || coalesce(description,'')) @@ plainto_tsquery($1) ORDER BY stars DESC LIMIT 50"
    )
    .bind(&query)
    .fetch_all(&pool)
    .await?;
    Ok(tools)
}
```

### 스타일링

- Tailwind CSS (leptos용 `leptos-tailwind` 연동)
- 또는 vanilla CSS (MVP는 Tailwind 권장, 클래스 기반)

---

## 6. 디렉토리 구조

```
OnchainAI/
├── docs/
│   ├── MVP_DESIGN.md          ← 이 파일
│   ├── UI_UX_DESIGN.md        ← UI/UX 설계
│   ├── DESIGN.md              ← Stitch DESIGN.md (AI 에이전트용 디자인 토큰)
│   └── SECURITY.md            ← 보안 설계 (OWASP + SIWE + Supabase RLS)
├── migrations/
│   ├── 001_init.sql           ← tools/sources 스키마
│   ├── 002_auth.sql           ← profiles/siwx_sessions 스키마
│   └── 003_social.sql         ← comments/upvotes/bookmarks 스키마
├── src/
│   ├── main.rs                ← 진입점 (서버 + 스케줄러 시작)
│   ├── app.rs                 ← Leptos 앱 라우터
│   ├── config.rs              ← 환경변수, DB URL
│   ├── models/
│   │   ├── mod.rs
│   │   ├── tool.rs            ← Tool 구조체
│   │   ├── category.rs        ← Category 구조체
│   │   ├── user.rs            ← User/Profile 구조체 (nickname, bio, auth_method, avatar)
│   │   └── comment.rs         ← Comment/Upvote 구조체
│   ├── auth/
│   │   ├── mod.rs             ← 인증 미들웨어
│   │   ├── github.rs          ← GitHub OAuth (Supabase)
│   │   ├── email.rs           ← Email 매직링크 (Supabase)
│   │   └── siwx.rs            ← SIWX 지갑 인증 (CAIP-122)
│   ├── components/
│   │   ├── mod.rs
│   │   ├── search_bar.rs
│   │   ├── tool_card.rs
│   │   ├── category_grid.rs
│   │   ├── install_button.rs
│   │   ├── login_modal.rs     ← 3-way 로그인 모달
│   │   ├── comment_section.rs ← 댓글 + 업보트
│   │   └── bottom_sheet.rs    ← 모바일 바텀 시트
│   ├── pages/
│   │   ├── mod.rs
│   │   ├── home.rs            ← 홈 페이지
│   │   ├── tools_list.rs      ← 전체 도구 리스트
│   │   ├── tool_detail.rs     ← 도구 상세 (관리자: 인라인 에디터)
│   │   ├── category.rs        ← 카테고리별
│   │   ├── submit.rs          ← 도구 등록 폼
│   │   ├── auth.rs            ← 로그인 콜백 페이지
│   │   ├── settings.rs        ← 유저 프로필 설정
│   │   └── admin/
│   │       ├── mod.rs         ← 관리자 라우트 보호 (is_admin 체크)
│   │       ├── dashboard.rs   ← 관리자 대시보드
│   │       ├── tools.rs       ← 도구 관리 (승인/거절/수정/삭제)
│   │       ├── categories.rs  ← 카테고리 관리
│   │       ├── users.rs       ← 유저 관리 (밴/관리자 권한)
│   │       ├── comments.rs    ← 댓글 관리 (삭제)
│   │       ├── settings.rs    ← 사이트 설정
│   │       └── crawler.rs     ← 크롤러 제어 (상태/수동 실행)
│   ├── server/
│   │   ├── mod.rs
│   │   ├── functions.rs       ← Leptos server functions (DB 쿼리)
│   │   └── mcp.rs             ← MCP 서버 (rmcp 핸들러)
│   └── crawler/
│       ├── mod.rs             ← 오케스트레이터
│       ├── scheduler.rs       ← tokio-cron-scheduler
│       ├── normalizer.rs      ← 정규화
│       ├── deduper.rs         ← 중복 제거
│       └── sources/
│           ├── mod.rs         ← SourceCrawler 트레이트
│           ├── cryptoskill.rs
│           ├── web3mcp.rs
│           ├── github.rs      ← 신규 발견 + star 동기화 + self_register
│           └── npm.rs
├── templates/
│   └── siwx_message.txt       ← SIWX 서명 메시지 템플릿
├── Cargo.toml
├── Dockerfile
├── style/                     ← Tailwind CSS
├── .env.example               ← 환경변수 템플릿 (실제 .env는 gitignore)
└── README.md                  ← GitHub 리포 소개 (별 받기용)
```

### Cargo.toml (주요 의존성)

```toml
[dependencies]
# 웹 프레임워크
leptos = { version = "0.7", features = ["ssr", "nightly"] }
leptos_axum = "0.7"
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["fs", "cors"] }

# MCP
rmcp = { version = "0.1", features = ["server", "transport-sse"] }

# DB
sqlx = { version = "0.8", features = ["postgres", "uuid", "chrono", "json", "macros", "runtime-tokio"] }

# 크롤링
reqwest = { version = "0.12", features = ["json"] }
scraper = "0.22"
tokio = { version = "1", features = ["full"] }
tokio-cron-scheduler = "0.14"
async-trait = "0.1"

# 직렬화
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 유틸
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
slug = "0.1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
dotenvy = "0.15"

# 인증
supabase-auth = "0.3"
jsonwebtoken = "9"
ethers-core = "2"
solana-sdk = "2"

# 보안
validator = { version = "0.18", features = ["derive"] }
governor = "0.6"               # Rate limiting
argon2 = "0.5"                 # 비밀번호 해싱 (나중용)
axum_csrf = "0.1"              # CSRF 보호 (선택적)
```

---

## 7. 단일 바이너리 구조

하나의 Rust 바이너리가 3개 역할을 동시에 실행:

```rust
// src/main.rs (개념)
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::init();

    let pool = setup_db().await?;
    run_migrations(&pool).await?;

    // 크롤 스케줄러 (백그라운드 태스크)
    let crawler_pool = pool.clone();
    tokio::spawn(async move {
        crawler::start_scheduler(crawler_pool).await;
    });

    // Leptos SSR + Axum 서버 (웹사이트 + MCP 엔드포인트)
    let app = build_app(pool);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## 8. 빌드 순서

```
1. GitHub 리포 생성 + README.md + .gitignore            (15분)
2. Cargo 프로젝트 세팅 + 의존성 정리                    (30분)
3. Supabase Postgres 생성 + 마이그레이션 전체           (1시간)
   (001_init.sql + 002_auth.sql + 003_social.sql)
4. DB 모델 + sqlx 연결                                   (1시간)
5. 인증: Supabase Auth 연동 (GitHub + Email)             (2시간)
6. 인증: SIWX 지갑 인증 (CAIP-122 + 서명 검증)           (3시간)
7. 크롤러: CryptoSkill 소스 구현                         (2시간)
8. 크롤러: web3-mcp-hub 소스 추가                        (1시간)
9. 크롤러: GitHub topics + star 동기화 + self_register   (2시간)
10. 크롤러: npm 소스 + 스케줄러 + 정규화/중복제거        (2시간)
11. Leptos: 홈 + 카테고리 그리드 + DESIGN.md 적용        (2시간)
12. Leptos: 도구 리스트 + 상세 + 서버펑션                (3시간)
13. Leptos: 사이드바 필터 + 미리보기 패널 + 바텀 시트    (3시간)
14. Leptos: 댓글 + 업보트 + 북마크 + 로그인 모달         (2시간)
15. Leptos: 관리자 패널 (대시보드+도구승인+카테고리+유저+설정+크롤러) (4시간)
16. MCP 서버 (rmcp) 구현 + Axum 마운트                   (2시간)
16. Dockerfile + 배포 (Railway)                          (1시간)
17. 초기 데이터 크롤 실행 + 검증                         (1시간)
                                                       ──────
                                                       약 32시간
```

> 관리자 패널 추가로 +4시간 (대시보드, 도구 승인, 카테고리/유저/설정/크롤러 관리).
> 인증 시스템 +11시간 (SIWX 3h + Supabase Auth 2h + 댓글/업보트 2h 등).
> UI 컴포넌트 세분화 +3시간 (바텀 시트, 미리보기 패널, 로그인 모달).

---

## 9. 배포

### Dockerfile

```dockerfile
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/onchainai /app/onchainai
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/style /app/style
ENV DATABASE_URL=postgresql://...
EXPOSE 3000
CMD ["/app/onchainai"]
```

### Railway

Railway = "Vercel인데 Rust/Docker/상시실행도 되는 버전". **`main`** 브랜치 push 시 자동 빌드/배포(또는 `./scripts/deploy-railway.sh`), 상시 실행, 자동 HTTPS.

```dockerfile
# Dockerfile (동일)
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/onchainai /app/onchainai
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/style /app/style
ENV DATABASE_URL=postgresql://...
EXPOSE 3000
CMD ["/app/onchainai"]
```

```
# Railway 설정 (Dockerfile — railway.json)
# 1. Railway 프로젝트 생성
# 2. GitHub 저장소 연결 → `main` 브랜치 자동 빌드/배포
# 3. 환경변수 설정: DATABASE_URL, PORT=3000
# 4. 도메인 연결: www.onchain-ai.xyz (Railway 자동 HTTPS)
```

> 주의: Vercel 서버리스 불가 (Rust 단일 바이너리 + 상시 크롤 스케줄러).
> Railway 상시 실행 컨테이너 사용. 약 $5/월 (무료티어 있음).
> DB는 Supabase Postgres 외부 호스팅 유지 (Railway에 DB 올릴 수도 있지만 Supabase가 무료티어 좋음).

---

## 10. x402 수익 모델 (단계별)

### OnchainAI 수익 vs 유저 수익 분리

- **유저 수익**: 자기 x402 엔드포인트를 등록 → agent가 직접 호출 → 100% 수취
- **OnchainAI 수익**: 등록/노출/검증 과정에서 수수료

### 3단계 수익 구조

| 단계 | 구조 | 방식 | 수익 | 인프라 |
|---|---|---|---|---|
| **MVP** | 등록료 | x402 툴 등록 시 1회 x402 결제 | 건당 $5 | 가벼움 |
| **MVP+** | Featured 배치 | 상단 노출, 기간당 x402 과금 | $10/주 | 가벼움 |
| **MVP+** | 검증 배지 | 검증 테스트 통과 시 배지, 유지료 | $20/월 | 중간 |
| **성장기** | 프리미엄 검색 | OnchainAI MCP 프리미엄 쿼리 x402 과금 | $0.01/call | 중간 |
| **성숙기** | 라우팅 수수료 | x402 툴 랩핑, 호출당 마진 | $0.001/call | 무거움 |

### MVP 수익 (등록료만)

- 일반 MCP/CLI/SDK/API 등록: **무료** (트래픽 모으기)
- x402 툴 등록: **$5/등록** (x402 결제)
  - x402 쓰는 유저는 이미 결제 인프라 보유 → 마찰 적음
- 검증 배지 신청: **$10/신청** (x402 결제, MVP+에서 추가)

### 성숙기 수익 (라우팅 수수료)

```
agent → OnchainAI MCP → 등록된 x402 툴 호출
  원가 $0.003 → OnchainAI가 $0.004로 랩핑
  $0.003 유저 수취, $0.001 OnchainAI 마진
```

- 필요 부가가치: 스키마 정규화 + 비교 + 추천 + 페일오버
- 없으면 agent가 Zapper 직접 호출 → OnchainAI 거칠 이유 없음
- 프리미엄 검색(구조 3)과 라우팅(구조 2)이 합쳐져야 의미

### 수익 규모 시나리오

| 시나리오 | 등록 | Featured | 라우팅 호출 | 연 수익 |
|---|---|---|---|---|
| 보수적 | 100건 × $5 | 10개 × $10/주 | 0 | $500 + $5,200 = $5,700 |
| 성장 | 500건 × $5 | 30개 × $10/주 | 100만 × $0.001 | $2,500 + $15,600 + $1,000 = $19,100 |
| 스케일 | 2,000건 × $5 | 50개 × $10/주 | 1,000만 × $0.001 | $10,000 + $26,000 + $10,000 = $46,000 |

---

## 11. 다음 단계 (MVP 이후)

1. **셀프등록 UI** — 프로젝트가 직접 도구 등록/클레임
2. **x402 등록 결제** — x402 툴 등록 시 x402로 등록료 결제
3. **검증 배지** — 실제 MCP 호출 테스트 자동화 + 배지 유료화
4. **Featured 배치** — 상단 노출 유료화
5. **추천엔진** — "이걸 하고 싶다" → 최적 도구 추천
6. **프리미엄 검색** — OnchainAI MCP 프리미엄 쿼리 x402 과금
7. **x402 라우팅 게이트웨이** — 통합 라우팅 + 호출당 마진
8. **별도 CLI** — 터미널에서 검색/설치 (Rust clap)
9. **Typesense 검색** — 규모 커지면 전환
