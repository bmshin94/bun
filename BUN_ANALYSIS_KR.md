# Bun 저장소 분석 & 활용 가이드 (한국어)

> 이 문서는 `bmshin94/bun` 저장소를 분석하고, 활용 방안과 수익화 아이디어를
> 정리한 대화 내용을 기록한 문서입니다.
>
> - **작성일**: 2026-09-19
> - **분석 대상**: https://github.com/bmshin94/bun (fork)
> - **원본 저장소**: https://github.com/oven-sh/bun
> - **공식 문서**: https://bun.com/docs

---

## 목차

1. [저장소 정체 파악](#1-저장소-정체-파악)
2. [Bun 쉽게 이해하기](#2-bun-쉽게-이해하기)
3. [자주 묻는 질문 7가지](#3-자주-묻는-질문-7가지)
4. [수익화 아이디어](#4-수익화-아이디어)
5. [참고 링크](#5-참고-링크)

---

## 1. 저장소 정체 파악

### 1.1 결론

이 저장소는 **Bun** — 올인원 JavaScript/TypeScript 런타임 & 툴킷의 **전체 소스코드**입니다.
원본 `oven-sh/bun` 을 fork 한 것이며, fork 이후 `CLAUDE.md` 에 페르소나 가이드가
추가된 커밋(`3fd34fb6`)이 존재합니다.

### 1.2 실측 스펙

| 항목 | 값 |
| --- | --- |
| 총 파일 수 | 19,810개 |
| 저장소 용량 | 323MB |
| Rust 파일 | 1,537개 |
| C++/헤더 파일 | 1,369개 |
| TypeScript 파일 | 3,464개 |
| Zig 파일 | 0개 (Rust 전환 완료) |
| 버전 | v1.4.3 |
| 라이선스 | MIT (JavaScriptCore 부분만 LGPL-2) |

### 1.3 Bun이 대체하는 것

기존 Node.js 생태계에서 개별 도구로 쓰던 것들을 실행파일 하나로 통합합니다.

| 기존 도구 | Bun 대응 |
| --- | --- |
| Node.js | `bun run` |
| npm / yarn / pnpm | `bun install`, `bun add` |
| npx | `bunx` |
| Webpack / Vite / esbuild | `bun build` |
| Jest / Vitest | `bun test` |
| ts-node / tsx | 네이티브 TS 실행 |
| dotenv | `.env` 자동 로딩 |

```bash
bun run index.tsx                      # TS/JSX 바로 실행
bun install                            # 패키지 설치
bun test                               # 테스트
bun build ./index.ts --outdir ./dist   # 번들링
bunx cowsay 'Hello'                    # 패키지 실행
```

### 1.4 폴더 구조

```
src/                    핵심 소스 (Rust + C++)
├── runtime/
│   ├── server/         Bun.serve (HTTP 서버)
│   ├── node/           Node.js 호환 레이어
│   ├── webcore/        fetch, streams, Blob, Response
│   ├── crypto/         WebCrypto, node:crypto
│   └── test_runner/    bun test 엔진
├── install/            패키지 매니저
├── bundler/            번들러
├── js_parser/          JS/TS 파서 (직접 구현)
├── js_printer/         코드 출력기
├── shell/              크로스플랫폼 쉘
├── sql/                Postgres / MySQL / SQLite
├── css/                CSS 파서
├── http/               HTTP 클라이언트 + WebSocket
├── bake/               SSR / 개발서버
└── jsc/                JavaScriptCore 바인딩

docs/                   공식 문서 원본
test/                   테스트 스위트
packages/               VSCode 확장, 타입 정의, 플러그인
bench/                  성능 벤치마크
scripts/                빌드/CI 자동화
.claude/                Claude Code 에이전트 설정  ★ 핵심
```

### 1.5 기술적 차별점

1. **엔진**: Node.js는 V8, Bun은 **JavaScriptCore**(Safari 엔진). 시작 속도가 빠름.
2. **구현 언어**: Rust (원래 Zig → 전면 재작성 완료).
3. **배터리 포함**: `bun:sqlite`, `Bun.s3`, `Bun.redis`, `Bun.sql`, `Bun.$` 내장.

---

## 2. Bun 쉽게 이해하기

### 2.1 비유: 개별 도구 vs 시스템 키친

- **Node.js 방식**: 냄비, 가스레인지, 칼, 도마를 각각 다른 회사에서 사서 조립
- **Bun 방식**: 전부 들어있는 시스템 키친 하나

### 2.2 속도가 빠른 이유 3가지

**① 엔진 선택**

| 엔진 | 특성 | 유리한 상황 |
| --- | --- | --- |
| V8 (Node) | 오래 돌릴수록 최적화 | 장시간 서버 |
| JavaScriptCore (Bun) | 즉시 빠름 | CLI, 서버리스, 짧은 실행 |

**② 트랜스파일 단계 제거**

```
기존:  TS → ts-node 변환 → JS → Node 실행   (2단계)
Bun:   TS → Bun 실행                        (1단계)
```

**③ 하드링크 기반 설치**

- npm: 파일을 하나씩 복사
- Bun: OS 하드링크 사용 + 바이너리 lockfile(`bun.lock`)

### 2.3 `.claude/` 폴더 구조 (가장 실용적인 부분)

```
.claude/
├── settings.json              Hook 설정
├── hooks/
│   ├── pre-bash-guard.js      위험 명령어 차단 (PreToolUse)
│   └── post-edit-format.js    편집 후 자동 포맷팅 (PostToolUse)
├── commands/                  슬래시 명령어 6개
│   ├── dedupe.md
│   ├── find-issues.md
│   ├── find-duplicate-prs.md
│   ├── upgrade-boringssl.md
│   ├── upgrade-nodejs.md
│   └── upgrade-webkit.md
├── skills/                    에이전트 스킬 8개
│   ├── verify/
│   ├── implementing-jsc-classes-cpp/
│   ├── implementing-jsc-classes-rust/
│   ├── javascriptcore-garbage-collector/
│   ├── rust-system-calls/
│   ├── slowest-tests/
│   ├── writing-bundler-tests/
│   └── writing-dev-server-tests/
└── docs/landing-prs.md

CLAUDE.md   (19KB)  AI에게 주는 프로젝트 규칙서
REVIEW.md   (26KB)  머지된 PR 약 2,500개에서 추출한 리뷰 규칙
```

실제 `settings.json` 내용:

```json
{
  "hooks": {
    "PreToolUse": [
      { "matcher": "Bash",
        "hooks": [{ "type": "command",
                    "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/pre-bash-guard.js" }] }
    ],
    "PostToolUse": [
      { "matcher": "Write|Edit|MultiEdit",
        "hooks": [{ "type": "command",
                    "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/post-edit-format.js" }] }
    ]
  }
}
```

### 2.4 이 저장소에서 얻을 수 있는 것

| # | 항목 | 설명 |
| --- | --- | --- |
| 1 | 빠른 개발 도구 | Bun 자체를 설치해서 사용 |
| 2 | **AI 에이전트 설정 레퍼런스** | `.claude/` 구조를 내 프로젝트에 이식 |
| 3 | 시스템 프로그래밍 학습 자료 | 파서/번들러/패키지매니저 내부 구현 |

---

## 3. 자주 묻는 질문 7가지

### Q1. 설치 및 사용법

**A. 일반 사용 (저장소 불필요)**

```bash
# macOS / Linux
curl -fsSL https://bun.com/install | bash

# Windows
powershell -c "irm bun.sh/install.ps1 | iex"

# npm
npm install -g bun

# Homebrew
brew tap oven-sh/bun && brew install bun

# Docker
docker pull oven/bun
```

기본 명령어:

```bash
bun --version
bun init                  # 새 프로젝트
bun add react             # 패키지 설치
bun run index.ts          # 실행
bun test                  # 테스트
bun build ./index.ts --outdir ./dist
bun --hot server.ts       # 핫리로드
bun repl                  # REPL
bun upgrade               # 업데이트
```

**B. 소스 빌드 (이 저장소)**

요구 사항: 빌드 시간 30분~2시간, 디스크 20~30GB, RAM 16GB 이상 권장.

```bash
git clone https://github.com/bmshin94/bun
cd bun
bun install
bun bd                                 # 디버그 빌드 → ./build/debug/bun-debug
bun bd run script.ts                   # 빌드된 바이너리로 실행
bun bd test test/js/bun/http/serve.test.ts
bun run build:release                  # 릴리즈 빌드
```

> `CLAUDE.md` 규칙: `bun test` 직접 실행 금지. 반드시 `bun bd test` 사용.

### Q2. 플러그인 / 스킬 / MCP 중 무엇인가?

**셋 다 아님. Bun은 런타임(Runtime)입니다.** Node.js와 같은 계층입니다.

다만 혼동할 만한 이유가 있습니다.

| 개념 | Bun과의 관계 |
| --- | --- |
| 플러그인 | Bun은 플러그인을 **받는** 호스트 (`Bun.plugin()`). `packages/bun-plugin-svelte`, `bun-plugin-yaml` 존재 |
| 스킬 | Bun 기능이 아니라, 저장소 `.claude/skills/` 에 Claude Code용 스킬 8개가 **동봉**되어 있음 |
| MCP | Bun으로 MCP 서버를 **만들기 좋음** (빠른 시작 + 단일 바이너리 컴파일) |

### Q3. API 토큰이 필요한가?

**아니요.** Bun은 로컬 실행 프로그램이며 로그인/결제/토큰이 전혀 없습니다. MIT 라이선스 무료입니다.

토큰이 등장하는 예외 3가지 (모두 Bun 자체와 무관):

1. **Bun 개발자용 CI 스크립트** — `bun run ci:errors` 는 `BUILDKITE_API_TOKEN`, `bun run pr:comments` 는 GitHub 토큰 필요
2. **사설 npm 레지스트리** — `.npmrc` 인증 (npm/yarn도 동일)
3. **사용자가 만든 앱이 외부 API 호출** — 사용자 코드의 문제

참고로 Bun은 `.env` 자동 로딩과 OS 키체인 기반 `Bun.secrets` 를 내장하고 있습니다.

### Q4. 왜 GitHub에서 유명한가?

1. **속도로 화제몰이** — 2022년 공개 시 "Node.js보다 3배 빠른 HTTP 서버" 데모가 확산. README에 해당 트윗 배지가 아직 남아있음
2. **실제 고통 해결** — node_modules 용량, 번들러 설정, 도구 조립 문제를 통합으로 해소
3. **드롭인 대체 포지셔닝** — 새 언어 학습 불필요, 기존 Node 프로젝트에 바로 적용 가능
4. **자본과 전업 팀** — Oven 사에서 VC 투자를 받아 풀타임 개발
5. **기술적 야심** — 파서/번들러/패키지매니저/쉘 전부 직접 구현, V8 대신 JSC 선택, Zig → Rust 전면 재작성
6. **배터리 포함** — 버전마다 새 내장 기능(SQLite, S3, Redis, SQL, Shell)이 추가되며 지속적 화제

### Q5. 로컬 에이전트 구축에 도움이 되는가?

**두 가지 층위 모두에서 도움이 됩니다.**

**층위 1 — `.claude/` 폴더를 레퍼런스로 활용 (더 중요)**

| 구성요소 | 배울 점 |
| --- | --- |
| `CLAUDE.md` | 추상적 지시 대신 **"하지 마라" 리스트를 구체적으로** 제공 |
| `hooks/` | AI를 신뢰하는 대신 **가드레일로 감싸는** 구조 (사전 차단 + 사후 포맷팅) |
| `skills/` | 모든 지식을 프롬프트에 넣지 않고 **필요 시 로드**하는 모듈화 |
| `REVIEW.md` | 규칙을 감이 아니라 **실제 PR 데이터에서 추출** |
| `commands/` | 반복 작업을 **슬래시 명령어로 박제** |

`CLAUDE.md` 실제 규칙 예시:

```
**CRITICAL**: Never use `bun test` directly - it won't include your changes
**CRITICAL**: Do not write flaky tests. Do not use setTimeout to wait
**Be humble & honest** - NEVER overstate what you got done
If you need a paragraph-long comment to justify why the workaround is OK,
the code is wrong — fix the code.
```

**층위 2 — Bun을 에이전트 실행 환경으로 사용**

장점:

- **시작 속도** — 도구 호출이 잦은 에이전트에서 누적 이득
- **단일 실행파일 배포** — `bun build ./agent.ts --compile --outfile my-agent`
- **필요 부품 내장** — `bun:sqlite`(대화기록), `Bun.$`(쉘), `Bun.serve`(웹훅), `Bun.redis`(캐시), `.env` 자동 로딩
- **TS 그대로 실행** — 빌드 단계 없이 프로토타이핑

단점:

| 항목 | 현실 |
| --- | --- |
| Python 생태계 | LangChain, LlamaIndex 등은 Python이 압도적 |
| 로컬 LLM 바인딩 | llama.cpp, Ollama는 Python/Node가 더 성숙 |
| Node 호환성 | 대부분 동작하나 드물게 네이티브 모듈 이슈 가능 |

권장 조합:

```
두뇌     : Claude (API 또는 Claude Code)
실행     : Bun (에이전트 런타임, 단일 바이너리 배포)
제어     : Bun의 .claude/ 구조 벤치마킹
무거운 ML : 필요 시 Python 마이크로서비스로 분리
```

### Q6. 수익화 아이디어가 있는가?

있습니다. 핵심 원칙은 **"Bun 자체는 MIT 무료이므로 팔 수 없고, Bun 위에서 수익이 난다"** 입니다.
상세 내용은 [4. 수익화 아이디어](#4-수익화-아이디어) 참고.

### Q7. React나 PHP로 만들 수 있는가?

**해석 A — React/PHP로 Bun 같은 것을 만들 수 있는가? → 불가능**

계층이 다릅니다.

```
React / PHP 코드          ← 실행 당하는 쪽
─────────────────────
런타임 (Bun/Node/PHP엔진)  ← 실행 시키는 쪽
─────────────────────
OS → CPU
```

| 필요 기능 | React/PHP로 가능? |
| --- | --- |
| 메모리 직접 관리 | 불가 (GC가 관리) |
| 시스템콜 직접 호출 | 불가 (샌드박스) |
| JS 엔진(JSC) 임베딩 | 불가 (C++ 링킹 필요) |
| 네이티브 실행파일 컴파일 | 불가 |

그래서 Bun은 Rust + C++로 작성되었습니다. 만들려면 Rust/C++/Zig/Go 급 언어가 필요합니다.

**해석 B — React/PHP 프로젝트에서 Bun을 쓸 수 있는가? → 가능, 오히려 권장**

React:

```bash
bun create vite my-app --template react-ts
bun install
bun run dev

bunx create-next-app@latest my-next-app
```

Bun 단독 풀스택 (번들러 설정 0개):

```typescript
import index from "./index.html";

Bun.serve({
  port: 0,
  routes: {
    "/": index,                                    // React 자동 번들링
    "/api/users": () => Response.json([{ id: 1 }]),
  },
  development: true,                               // HMR 자동
});
```

PHP — 직접 실행은 불가하지만 협업 패턴은 일반적입니다.

1. **Laravel + Bun** (가장 흔함) — `bun install`, `bun run build` 로 프론트엔드 빌드
2. **API 게이트웨이** — Bun이 앞단에서 받고 레거시 경로만 PHP로 프록시
3. **쉘 호출** — `await $`php artisan migrate --force`.text()`

정리:

| 질문 | 답 |
| --- | --- |
| React로 Bun 만들기 | 불가 |
| PHP로 Bun 만들기 | 불가 |
| React에서 Bun 쓰기 | 가능 (권장) |
| Laravel에서 Bun 쓰기 | 가능 (이미 일반적) |
| Bun이 PHP 실행 | 불가 |
| Bun ↔ PHP 협업 | 가능 |
| Bun으로 에이전트 만들기 | 가능 (권장) |

---

## 4. 수익화 아이디어

### 4.1 대원칙

1. **Bun 자체는 상품이 아님** — MIT 라이선스, 누구나 무료
2. **골드러시에선 청바지를 판다** — 사용자에게 필요한 주변 가치를 판매
3. **진짜 무기는 Bun이 아니라 `.claude/` 구조** — Bun 사용자는 늘어나지만, AI 에이전트를 실전 수준으로 세팅할 줄 아는 사람은 아직 적음

### 4.2 Tier 1 — 즉시 시작 가능 (초기 자본 0원)

#### 아이디어 1. AI 에이전트 온보딩 컨설팅 (최우선 추천)

**문제**: 회사들이 Claude Code 등을 도입했으나, AI가 프로젝트 규칙을 몰라 엉뚱한 결과를 냄.

**해결/납품물**:

```
CLAUDE.md           고객사 전용 규칙서
.claude/
├── settings.json   Hook 설정
├── hooks/          위험 명령 차단 + 자동 포맷팅
├── commands/       팀 반복작업 5~10개
└── skills/         도메인 지식 모듈
REVIEW.md           과거 PR 분석 기반 리뷰 규칙
+ 온보딩 문서 및 교육
```

**차별화 메시지**: "감으로 만든 것이 아니라, GitHub 스타 수만 개 프로젝트가 실제로 쓰는 구조를 적용합니다. Bun은 머지된 PR 약 2,500개를 분석해 REVIEW.md를 만들었고, 동일한 방식으로 귀사 PR을 분석합니다."

**자동화 도구 (Bun 활용)**:

```typescript
import { $ } from "bun";
import { Database } from "bun:sqlite";
// 1. 과거 PR 리뷰 코멘트 수집
// 2. SQLite에 저장 (내장)
// 3. Claude API로 패턴 추출 → CLAUDE.md 생성
// 4. bun build --compile 로 실행파일 배포
```

**가격 설계 (예시)**:

| 패키지 | 내용 | 가격 |
| --- | --- | --- |
| Starter | CLAUDE.md + 기본 hook | 100~200만원 |
| Standard | + commands/skills + 교육 | 300~500만원 |
| Enterprise | + 맞춤 스킬 + 월 유지보수 | 500만원~ + 월정액 |

**시작 절차**: 본인 프로젝트 적용 → Before/After 수치 측정 → 블로그·영상 공개 → 첫 고객 할인가 + 후기 확보

#### 아이디어 2. 한국어 교육 콘텐츠

Bun 한국어 자료와 AI 에이전트 세팅 한국어 자료가 거의 없는 상태입니다.

콘텐츠 기획:

| # | 제목 | 훅 |
| --- | --- | --- |
| 1 | npm install 3분 → 3초 | 속도 충격 |
| 2 | webpack 설정 파일을 0개로 | 고통 해소 |
| 3 | Node 프로젝트를 Bun으로 10분 만에 이사 | 실용 |
| 4 | AI가 프로젝트 규칙을 지키게 만드는 법 | 핵심 |
| 5 | 유명 오픈소스의 .claude/ 폴더 해부 | 희소성 |
| 6 | Bun으로 MCP 서버 만들어 실행파일 배포 | 심화 |

수익 단계: 무료 공개(신뢰 축적) → 유료 강의(인프런/유데미) → 전자책/템플릿 → 기업 출강 → 컨설팅 인바운드

심화 기획: "Bun 저장소 코드 리딩 시리즈" (파서 구현, 패키지 매니저 성능 비결, JSC 바인딩 분석) — 한국어로 거의 없는 영역이며 시니어 타겟 브랜딩에 효과적.

#### 아이디어 3. 마이그레이션 대행 서비스

**단계 구성**:

1. 무료 진단 — Bun으로 만든 자동 스캐너가 호환성/위험파일/예상 속도 향상 리포트 (리드 수집 도구 역할)
2. 마이그레이션 실행 — lockfile 전환, 호환성 이슈 수정, CI/CD 전환, 성능 Before/After 리포트
3. 사후 지원 1개월

**리드 생성 도구 예시**:

```bash
bunx @scope/bun-migrate-check
# 호환 가능: 142개 파일
# 확인 필요: 3개
# 예상 install 속도: 18배 향상
# 상세 리포트 받기: [이메일 입력]
```

가격: 프로젝트 규모별 200~1,000만원

### 4.3 Tier 2 — 3~6개월 투자 필요

#### 아이디어 4. 오픈소스 → 스폰서십 / 기업판

Bun 생태계에서 비어있는 영역:

- `bun-plugin-*` (protobuf, graphql 등 아직 없는 로더)
- `Bun.sql` 기반 ORM/마이그레이션 툴
- Bun 전용 APM/모니터링
- **AI 에이전트 스캐폴딩 CLI** (추천)

```bash
bunx create-claude-setup
# ? 프로젝트 타입 (Next.js / Laravel / Python / ...)
# ? 팀 규모 (1-5 / 5-20 / 20+)
# ? 포맷터 (prettier / biome / ...)
# → CLAUDE.md, .claude/hooks/, .claude/commands/ 자동 생성
```

수익 구조: GitHub Sponsors → Pro 버전(팀 기능) → 기업 지원 계약 → 인지도 기반 컨설팅 유입

#### 아이디어 5. Bun 네이티브 SaaS

Bun의 강점이 곧 상품 경쟁력이 되는 영역:

| 서비스 | Bun 강점 |
| --- | --- |
| Webhook 중계/변환 | 빠른 콜드스타트 = 서버비 절감 |
| 이미지 변환 API | `src/image/` 내장 처리 |
| 코드 실행 샌드박스 | 시작 속도가 곧 UX |
| AI 에이전트 호스팅 | 단일 바이너리 배포 |

구체 예시 — **팀 AI 규칙 관리 SaaS**: 웹 대시보드에서 팀 AI 규칙 중앙 관리, GitHub 앱으로 저장소 자동 동기화, 규칙 위반 통계, 실제 리뷰 코멘트 기반 신규 규칙 추천. 가격 팀당 월 $29~$99. 기술 스택은 `Bun.serve` + `bun:sqlite` + `Bun.s3` 로 의존성 최소화.

주의: 이 단계는 Tier 1 컨설팅을 하면서 실제 고객 니즈를 확인한 후 착수하는 것이 안전합니다.

### 4.4 Tier 3 — 장기 / 고위험

- **엔터프라이즈 지원 사업** — SLA 기반 기술지원, 24/7 대응, 보안 패치 백포팅. 연 단위 계약. Bun 내부 코드 이해 필요.
- **오픈소스 기여 → 개인 브랜딩** — 직접 수익은 없으나 몸값 상승. `REVIEW.md`, `CONTRIBUTING.md` 부터 시작.

### 4.5 로드맵

| 기간 | 할 일 | 예상 수익 |
| --- | --- | --- |
| 1~2개월 | 내 프로젝트에 `.claude/` 적용, Before/After 측정, 콘텐츠 3~6편 발행 | 0원 |
| 3~4개월 | 무료 도구 배포, 첫 컨설팅(할인가), 사례·후기 확보 | 100~300만원 |
| 5~8개월 | 컨설팅 정가 영업, 강의 오픈, 기업 세미나 | 월 300~800만원 |
| 9~12개월 | 컨설팅에서 발견한 니즈로 SaaS 개발, 구독 수익 시작 | 월 500만원~ + MRR |

### 4.6 최우선 추천

**아이디어 1 (AI 에이전트 온보딩 컨설팅)**

| 기준 | 평가 |
| --- | --- |
| 초기 자본 | 0원 |
| 시작 속도 | 즉시 가능 |
| 시장 타이밍 | AI 도입 붐 정점 |
| 경쟁 강도 | 아직 낮음 |
| 레퍼런스 | Bun 저장소가 이미 확보됨 |
| 확장성 | 교육 → 도구 → SaaS 로 자연 확장 |

> 참고: 위 가격·수익 수치는 시장 상황에 따라 달라지는 **예시 추정치**이며, 실제 검증된 실적이 아닙니다.

---

## 5. 참고 링크

### 저장소

- 이 저장소 (fork): https://github.com/bmshin94/bun
- 원본 저장소: https://github.com/oven-sh/bun
- 이슈 트래커: https://github.com/oven-sh/bun/issues
- 로드맵: https://github.com/oven-sh/bun/issues/159
- Canary 빌드: https://github.com/oven-sh/bun/releases/tag/canary

### 문서

- 공식 문서: https://bun.com/docs
- 설치 가이드: https://bun.com/docs/installation
- 퀵스타트: https://bun.com/docs/quickstart
- Node.js 호환성: https://bun.com/docs/runtime/nodejs-compat
- 플러그인: https://bun.com/docs/runtime/plugins
- 번들러: https://bun.com/docs/bundler/index
- 단일 실행파일: https://bun.com/docs/bundler/executables
- 테스트 러너: https://bun.com/docs/test/index

### 커뮤니티

- Discord: https://bun.com/discord

### 저장소 내부 참고 문서

- `README.md` — 전체 기능 목록 및 문서 링크
- `CLAUDE.md` — AI 에이전트용 프로젝트 규칙 (19KB)
- `REVIEW.md` — PR 리뷰 규칙 (26KB)
- `CONTRIBUTING.md` — 빌드 및 기여 가이드 (16KB)
- `.claude/` — 에이전트 설정 (hooks / commands / skills)
- `docs/` — 공식 문서 원본
- `bench/` — 성능 벤치마크
