# Split Deploy Automation — Vercel Preview + Railway main-only

> Related: [[../../BUILD_DEPLOY_RULES]] | [[../../MCP_AGENT_WORKFLOW]] | [[../../../AGENTS.md]]
>
> Date: 2026-07-05 (v1.0)  
> Status: **Final spec — implemented**  
> Scope: 프로덕션 배포 경로 단일화, 추가 비용 없음, 에이전트/오너 혼동 방지

---

## 0. 문제

| # | 증상 | 원인 |
|---|------|------|
| D1 | feature branch 수동 `deploy-railway.sh` 후 prod ≠ `main` | CLI `railway up`이 **로컬 브랜치**를 prod에 올림 |
| D2 | Vercel은 push마다 preview인데 Railway는 수동 | split deploy 규칙이 문서에만 있고 강제 없음 |
| D3 | `frontend/`만 수정해도 API 재빌드 가능 | Railway watch paths 미설정 |
| D4 | staging/PR Railway 환경 없이 API 프리뷰 기대 | 추가 비용 없이는 불가 — 로컬 API로 대체 |

---

## 1. 목표

1. **Vercel**: 유지 — 모든 push → Preview; `main` push → Production.
2. **Railway**: `main` push + **API 경로 변경 시에만** 자동 배포 (추가 서비스/환경 **없음**).
3. **수동 배포**: env sync·긴급 핫픽스만; non-`main`은 `--force-non-main` 없으면 **거부**.
4. 에이전트가 “재배포” 요청 시 **기본 = main 머지 대기**, 수동은 예외.

---

## 2. 비목표

- Railway staging / PR ephemeral 환경 (추가 비용)
- GitHub Actions 자동 prod deploy (CI는 `workflow_dispatch` 유지)
- Vercel 동작 변경

---

## 3. 확정 아키텍처

```
feature branch push
  ├─ Vercel  → Preview URL (frontend)
  └─ Railway → (배포 없음)

main push
  ├─ Vercel  → Production (www.onchain-ai.xyz)
  └─ Railway → watchPatterns 매칭 시에만 Production API 빌드
```

### 3.1 Railway watch patterns (`railway.json`)

다음 경로 변경 시에만 Railway 빌드 트리거:

- `src/**`
- `migrations/**`
- `Dockerfile.api`
- `Cargo.toml`, `Cargo.lock`
- `railway.json`

`frontend/**`, `docs/**`, `scripts/**`(API 무관), 스펙만 변경 → **Railway 스킵**.

### 3.2 GitHub source

- Repo: `Coinyak/onchainai`
- Branch: **`main` only**
- 설정: `./scripts/configure-railway-git-deploy.sh` (idempotent)

### 3.3 수동 스크립트 역할

| 스크립트 | 용도 |
|----------|------|
| `configure-railway-git-deploy.sh` | GitHub `main` 연결 + watch paths 확인 |
| `deploy-railway.sh` | env sync; **main**에서만 기본 deploy |
| `deploy-railway.sh --vars-only` | env만 동기화 (빌드 없음) |
| `deploy-railway.sh --force-non-main` | 긴급 예외 (오너 명시 시만) |
| `post-deploy-verify.sh` | prod 스모크 (수동 배포·머지 후) |

---

## 4. 개발자 워크플로

| 작업 | 로컬 | Preview | Production |
|------|------|---------|------------|
| UI only | `dev-watch.sh` | Vercel preview (push) | `main` merge |
| API only | `cargo test` + local API | — (preview는 prod API 프록시) | `main` merge |
| Full-stack | `dev-watch.sh` | Vercel preview + local API | `main` merge |
| Docs only | — | — | 배포 없음 |

**API 변경을 preview URL에서 검증하려면** `dev-watch.sh`로 로컬 API를 띄우거나, `main` 머지 후 Railway 자동 배포를 기다린다.

---

## 5. 에이전트 규칙

1. **기본**: `deploy-railway.sh` / `vercel deploy --prod`를 feature branch에서 **실행하지 않음**.
2. **예외**: 사용자가 “지금 prod에 올려”라고 명시 → `main` 머지 우선 제안; 불가 시 `--force-non-main` + 사후 `main` 머지 안내.
3. **재배포 요청** = 대부분 `main`에 머지하면 Vercel/Railway가 각자 처리.
4. Migration 변경은 **반드시** prod `_sqlx_migrations`와 파일 번호 정합 (031–033 패턴).

---

## 6. 수용 기준

- [x] `railway.json`에 `watchPatterns` 정의
- [x] Railway service source = `Coinyak/onchainai` @ `main`
- [x] `deploy-railway.sh`가 non-`main`에서 exit 1 (unless `--force-non-main`)
- [x] `docs/BUILD_DEPLOY_RULES.md` §3에 본 스펙 링크
- [x] `AGENTS.md` Topic Routing에 스펙 1줄

---

## 7. 복구 (prod/main 불일치 시)

```bash
git checkout main && git pull
# feature PR 머지
# main push → Vercel prod + Railway (API paths only) 자동
./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz
```

수동 `railway up`으로 올린 코드는 **머지 없이는** `main` 자동 배포로 덮어쓰이지 않을 수 있음 — **머지가 정본**.