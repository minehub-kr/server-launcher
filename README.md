# Minehub Server Launcher

Tauri 기반 Minecraft 서버 실행기입니다. Rust 백엔드에서 서버 다운로드, 실행, 설정 파일 관리, 콘솔 명령, 플러그인 설치 같은 운영 기능을 처리합니다.


<img width="1392" height="932" alt="Demo Screenshot" src="https://github.com/user-attachments/assets/5ee0ecba-bf5f-4df0-8039-7d476d96c7cf" />

## 주요 기능

- 멀티 서버 프로필 관리
- Paper, Folia, Purpur, Vanilla 서버 버전 선택
- 구현체별 지원 버전 목록 조회
- Minecraft 버전에 맞는 Java 요구 버전 확인
- 서버 폴더 선택 및 바로 열기
- 서버 실행, 중지, 콘솔 명령 전송
- 콘솔 로그 실시간 반영 및 자동 스크롤
- 접속 중인 플레이어 목록 표시
- 플레이어 킥, 밴, OP, 화이트리스트, 게임 모드 변경 명령
- `server.properties` 폼 편집
- Paper/Spigot/Bukkit/Purpur YAML 설정 폼 및 원문 편집
- 설정 저장 전 백업 생성
- `ops.json`, `whitelist.json`, `banned-players.json` 관리
- 화이트리스트 켜기/끄기
- Modrinth 플러그인 검색 및 설치
- 플러그인 활성화/비활성화
- 월드/설정/플러그인 수동 백업
- 라이트/다크/시스템 테마

## 기술 스택

- Tauri 2
- Rust
- Nuxt 4
- Vue 3
- Nuxt UI
- TypeScript
- pnpm

## 요구 사항

- Node.js
- pnpm
- Rust toolchain
- Tauri 2 개발 환경
- Minecraft 서버 실행용 Java

Java는 현재 로컬에 설치된 런타임을 탐색하고 선택하는 방식입니다. Java 자동 다운로드는 아직 포함되어 있지 않습니다.

## 설치

```bash
pnpm install
```

## 개발 실행

```bash
pnpm run tauri:dev
```

프론트엔드만 확인할 때:

```bash
pnpm run dev
```

## 빌드

Nuxt 빌드:

```bash
pnpm run build
```

Tauri 앱 빌드:

```bash
pnpm run tauri:build
```

Tauri 빌드 설정은 `src-tauri/tauri.conf.json`에 있으며, 프로덕션 빌드 전 `pnpm generate`를 실행하도록 구성되어 있습니다.

## 검증

TypeScript 검사:

```bash
pnpm run typecheck
```

Rust 검사:

```bash
cd src-tauri
cargo check
```

Rust 테스트:

```bash
cd src-tauri
cargo test
```

## 프로젝트 구조

```text
app/
  components/              Vue 화면 컴포넌트
  composables/             런처 상태 및 액션
  pages/                   Nuxt 페이지
  types/                   프론트엔드 타입/상수
src-tauri/src/
  backup.rs                백업 생성
  config.rs                서버 설정과 권한/차단 파일 관리
  java.rs                  Java 런타임 탐색
  models.rs                공용 Rust 모델
  plugins.rs               Modrinth 검색/설치 및 플러그인 파일 관리
  runtime.rs               서버 실행, 콘솔, 상태, 로그 파싱
  settings.rs              앱 설정과 프로필 저장
  system.rs                공통 시스템 유틸리티
  versions.rs              서버 버전/다운로드 계획
```

## 데이터와 설정

앱 설정과 프로필 목록은 Tauri store를 통해 저장됩니다. 각 서버 프로필은 독립적인 서버 폴더, 서버 구현체, Minecraft 버전, Java 경로, 메모리, 설정 값을 가집니다.

서버 설정 저장 시 기존 설정 파일은 `config-backups/` 아래에 백업됩니다.

## 네트워크 사용

다음 기능은 네트워크 연결이 필요합니다.

- PaperMC/Folia 버전 및 빌드 조회
- Purpur 버전 및 빌드 조회
- Mojang 버전 매니페스트 조회
- Modrinth 플러그인 검색 및 다운로드
- Minecraft 닉네임 UUID 조회

## 현재 제약

- Java 자동 설치는 지원하지 않습니다.
- 플러그인 설치, 활성화, 비활성화는 서버 재시작 후 적용됩니다.
