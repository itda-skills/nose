# Upstream(corca-ai/nose) 경계 규칙

이 저장소는 corca-ai/nose의 포크(itda-skills/nose)이며, upstream과는 **읽기 전용** 관계다.

## 금지선 (절대 규칙)

1. **corca-ai/nose에 이슈·PR을 절대 생성하지 않는다.** push 권한도 없고, 시도도 하지 않는다.
2. **corca-ai/nose의 이슈·PR을 절대 구현하지 않는다.** 그 백로그(#653, #663, #657 등)는 corca-ai 측이 개발한다. 우리가 선행 구현하면 머지 충돌과 중복 작업만 남는다.
3. 커밋·push는 **`my`(itda-skills/nose)에만** 한다.

## 우리 작업 관리

- **우리 작업은 itda-skills/nose의 이슈로 직접 관리한다** (`gh issue list -R itda-skills/nose`).
- upstream 이슈·PR·코드는 방향 참고용으로 **읽기만** 한다.
- upstream 변경은 주기적으로 `git merge --no-ff origin/main`으로만 수용한다
  (SessionStart 훅이 드리프트를 감지해 알린다; 머지 후 워크스페이스 테스트 + 중복 게이트 재검증 필수 —
  중복 게이트 family ID는 span에 민감해 머지마다 재확인해야 한다).
