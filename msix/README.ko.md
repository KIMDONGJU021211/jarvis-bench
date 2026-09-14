# MSIX 안에서 데스크톱 에이전트가 기대는 것들 — 실측

> **English summary.** We measured what a full-trust desktop agent packaged as MSIX can still do, with a tiny
> std-only Rust probe. With the default manifest (no restricted capability): **newly created** files under AppData go
> to the package's private LocalCache (invisible to other apps, deleted on uninstall), but **edits to existing files**
> (overwrite, open-existing, temp-file + rename) land in the real file; writes into another package's
> `%LOCALAPPDATA%\Packages\X\LocalCache` and anywhere else under the user profile are real; a child process started from
> an exe **inside** the package shares the package's virtual view, while a child started from an exe **outside** it
> (e.g. `cmd.exe`) does not; loopback to an unpackaged local server works. Disabling virtualization needs
> `unvirtualizedResources`, which the Store approves per submission and describes as intended for certain games.
> Also: a shell launched by an MSIX-packaged app (here, the Claude desktop app) inherits that app's virtualization —
> measuring AppData from it measures *that app's* view.

2026-09-15, Windows 11 Pro 26200, 개발자 모드 · 느슨한 파일 등록(서명 없음). **스토어 인증을 받은 패키지가 아니다.**
동작 규칙을 잰 것이지, 특정 앱이 스토어를 통과한다는 뜻이 아니다.

## 왜 쟀나

JARVIS 는 로컬에서 도는 에이전트다. 다른 도구(Claude·Cursor·Codex)의 설정 파일에 자기를 잇고, 그 도구들이 읽을
토큰 파일을 만들고, 파이썬 백엔드를 자식 프로세스로 띄우고, 로컬 서버와 루프백으로 말한다. MSIX 는 AppData 쓰기를
가상화하므로 **이 중 무엇이 깨지는지** 문서만으로는 알 수 없었다. 그래서 같은 동작을 하는 점검기를 만들어 패키지 안에서 돌렸다.

## 결과

A = 기본 매니페스트(`runFullTrust` 만). B = `desktop6:FileSystemWriteVirtualization=disabled` + `unvirtualizedResources`.

| 동작 | A (기본) | B (가상화 끔) |
|---|---|---|
| AppData(Roaming·Local)에 **새 파일** | ❌ 앱 전용 LocalCache | ✅ 실제 경로 |
| AppData 기존 파일 고치기 — 덮어쓰기(`CREATE_ALWAYS`) | ✅ 실제 | ✅ 실제 |
| AppData 기존 파일 고치기 — 기존 열어 비우고 쓰기 | ✅ 실제 | ✅ 실제 |
| AppData 기존 파일 고치기 — 임시 파일 + 이름 바꾸기 | ✅ 실제 | ✅ 실제 |
| 다른 패키지 폴더 `%LOCALAPPDATA%\Packages\X\LocalCache` 고치기·새 파일 | ✅ 실제 | ✅ 실제 |
| 사용자 폴더 아래(AppData 밖) 새 파일 | ✅ 실제 | ✅ 실제 |
| 다른 MSIX 앱(이 PC 의 Claude 데스크톱) 설정을 `%APPDATA%\앱이름` 으로 읽기 | ❌ 없음 (그 앱이 만든 파일은 그 앱의 `Packages\…\LocalCache` 에만 있음) | ❌ 같음 |
| 같은 설정을 `Packages\앱_*\LocalCache\Roaming\…` 로 읽기 | ✅ | ✅ |
| 패키지 **밖** exe 자식(`cmd.exe`) — 부모의 전용 파일이 보이나 | ❌ 안 보임 | ✅ (둘 다 실제에 있음) |
| 패키지 **밖** exe 자식 — 새 AppData 파일 | 실제 경로 | 실제 경로 |
| 패키지 **안** exe 자식 — 부모의 전용 파일이 보이나 | ✅ 보임 | — |
| 패키지 **안** exe 자식 — 새 AppData 파일 | 앱 전용 LocalCache | — |
| 루프백: 자기 자신 / 패키지 밖 로컬 서버 | ✅ / ✅ `HTTP/1.1 200` | ✅ / ✅ |

**우리 판단(데스크톱 에이전트 기준):** 기본(A)으로 간다. 막히는 것은 「새로 만든 AppData 파일이 밖에 안 보이고, 앱을 지우면
같이 지워진다」 하나다. 다른 앱이 읽을 파일(토큰 등)은 실제 디스크 경로(`Packages\<PFN>\LocalCache\…`)를 알려 주거나 AppData 밖에 두고,
지워지면 안 되는 사용자 기억은 사용자 폴더에 복제한다. B 는 스토어 문서가 「제출마다 승인」·「현재 특정 PC 게임용」이라고
적고 있어 고려하지 않는다(Windows 11 의 폴더 단위 제외도 같은 제한 기능이 필요하다).

## 틀릴 뻔한 것 — 재는 손이 남의 상자 안에 있었다

첫 스테이징 폴더를 `%LOCALAPPDATA%` 에 만들었는데, 우리 셸에서는 보이는 매니페스트를 Windows 설치 서비스가 「경로 없음」으로 거절했다.
셸의 부모 체인을 보니 **Claude 데스크톱 앱(MSIX)** 이 있었다. MSIX 앱이 띄운 프로세스는 그 앱의 가상화를 물려받으므로, 셸이
AppData 에 새로 만든 것은 전부 Claude 전용 칸으로 갔고, 셸이 확인하는 AppData 도 Claude 의 합쳐진 보기였다. 앞서 돌린
E2E 시험 데이터도 모르는 사이 그 칸에 들어가 있었다.

그래서 점검기는:
- 「이미 있는 파일」 씨앗을 **explorer.exe 로 띄운 프로세스**(패키지 밖)가 만든다
- 스테이징은 NTFS 이면서 AppData 밖(`%SystemDrive%\MsixProbeStage`). exFAT 드라이브는 느슨한 등록이 거부된다(0x80073CFD)
- 모든 확인을 명시적 경로로 한다

첫 판에서는 자식 프로세스 명령의 따옴표가 `\"` 로 넘어가 `cmd` 가 경로를 못 읽었는데, 패키지 결과로 읽을 뻔했다(패키지 밖에서 같은
실행 파일을 돌려 가렸다).

## 돌리기

```powershell
# 필요: Rust(cargo), Windows SDK(makeappx), Windows 개발자 모드
powershell -NoProfile -ExecutionPolicy Bypass -File msix\prepare.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File msix\run_probe.ps1 -Variant A
powershell -NoProfile -ExecutionPolicy Bypass -File msix\run_probe.ps1 -Variant B
```

점검기는 실제 앱 설정 파일은 **크기만 읽고** 쓰지 않는다. 시험 파일은 AppData·`%LOCALAPPDATA%\Packages\JarvisProbeFakePkg`·
사용자 폴더(`.JarvisMsix*`)에 남으니 끝나면 지운다.

## 출처 (2026-09-15 확인)

- 파일 가상화·`unvirtualizedResources`: https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization
- MSIX 스토어 재서명: https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements
- 스토어 정책: https://learn.microsoft.com/en-us/windows/apps/publish/store-policies
