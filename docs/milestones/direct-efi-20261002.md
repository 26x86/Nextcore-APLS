# NextCore direct EFI milestone / NextCore 직접 EFI 마일스톤

Current Status / 현재 상태: Original native Tahoe26 reaches launchd main, WindowServer and an actual Apple keyboard/mouse pairing screen in Recovery/BaseSystem without Linux. Input and usable Recovery utilities remain unverified, as do installed GUI and ARM/macOS27 qualification. All nine repositories have reviewable progress drafts; a new source-bound binary release is pending. / Linux 없는 원본 네이티브 Tahoe26이 launchd 본체·WindowServer·실제 Apple 키보드/마우스 연결 안내 화면까지 진행했습니다. 입력 반응·사용 가능한 복구 메뉴·설치 GUI·ARM/macOS27 제품 검증은 미완료입니다. 관련 9개 저장소의 진행 초안을 공개했으며 새 소스 연결 바이너리 릴리즈는 대기 중입니다.

Target State / 목표 상태: Direct standard EFI → target-matching LaunchD → interactive Recovery GUI and installed GUI. Update all nine related repositories and provide one source-bound installer, ISO and EFI preview at every milestone. / 표준 EFI에서 대상에 맞는 LaunchD, 조작 가능한 리커버리 GUI와 설치 GUI까지 직접 도달하는 것이 목표입니다. 매 마일스톤마다 관련 저장소 9개를 갱신하고 동일 소스의 인스톨러·ISO·EFI 프리뷰를 제공합니다.

Snapshot / 확인 시각: 2026-10-02T01:29:34+09:00 KST. Earlier run08/run09 observations below are historical checkpoints. / 아래 실행08/09 관찰은 이전 확인 기록입니다.

## This repository / 이 저장소

- Repository / 저장소: `26x86/Nextcore-APLS`.
- Role / 역할: Independently versioned source module / 독립 버전 소스 모듈.
- Remote main before this record / 기록 전 원격 main: `b8d29eb92fbbc0d369d09caedf61800bf44350a4`.
- Integrated source snapshot / 통합 소스 확인 버전: `69c5988cffc596773a3d70b3f006d9973d7f1444` in `26x86/NextCore-Stuff`.

The committed integration branch has no changed paths in this crate relative to the NextCore-Stuff base 59e41262; this documentation record does not alter its source or dependency pins. / 통합 브랜치에서 NextCore-Stuff 기준 커밋 59e41262 대비 이 크레이트의 변경 파일은 없습니다. 이 진행 기록은 소스나 의존성 핀을 바꾸지 않습니다.

## Reached stages / 도달한 단계

| Track / 경로 | Verified observation / 확인된 관찰 | Limit / 한계 |
| --- | --- | --- |
| Original native Tahoe 26 / 원본 네이티브 Tahoe 26 | Run10 actual WindowServer, RecoveryOS Agent and Apple pairing frame; native Darwin25/product26 target match / 실행10 실제 WindowServer·RecoveryOS Agent·Apple 연결 안내 화면·네이티브 대상 일치 | Input, usable Recovery utilities and installed GUI unverified / 입력·복구 메뉴 사용·설치 GUI 미검증 |
| Original ARM/macOS 27 / 원본 ARM/macOS 27 | Clean source 6bff94d 26-instruction EFI prefix validated per invocation / 깨끗한 소스 6bff94d에서 EFI 명령 26개 구간을 실행별로 검증 | MPIDR stop; authored DRAM/software platform, no userspace / MPIDR에서 중단. 자체 DRAM·소프트웨어 플랫폼이며 사용자 공간은 미확인 |
| Authored firmware configuration / 자체 펌웨어 설정 | Configuration 15/picker 6 fixture cases pass; packaged normal EFI additionally passes case 2 / 설정 15개·선택기 6개 사례 통과, 패키지의 일반 EFI도 사례 2 통과 | Firmware UI is separate from macOS Recovery / 펌웨어 UI이며 macOS 리커버리와 구분 |
| Matched local research bundle / 동일 소스 로컬 연구 묶음 | Three EFIs, ISO and ZIP structurally verified at clean c156d1f / 깨끗한 c156d1f에서 EFI 3개·ISO·ZIP 구조 검증 | New remote release and qualified installation pending / 새 원격 릴리즈와 제품 설치 검증은 대기 중 |

Later ARM EFI observations without a valid trace return are separate and do not supersede the validated source-6bff94d 26-instruction prefix. / 이후의 ARM EFI 관찰에서 유효한 추적 종료가 없었던 결과는 별도 증거이며, 소스 6bff94d에서 검증한 명령 26개 구간을 대체하지 않습니다.

Native runs use original Tahoe 26.6.2 (build 25G83), Darwin 25.6 and normal EFI from baseline source 59e41262, SHA-256 `acaecadd7e9d3ebec9699fb0ac57d7dc15bc4584a38db56d878e1cd41bca0e64`. Original inputs were preserved, backing remained read-only, writes used a private overlay and owned guests were reaped. They do not execute the newer packaged normal EFI. / 네이티브 실행은 원본 Tahoe 26.6.2(build 25G83)·Darwin 25.6과 기준 소스 59e41262의 일반 EFI를 사용했습니다. 원본 입력 보존, 읽기 전용 원본, 비공개 쓰기 오버레이와 게스트 정리를 확인했습니다. 새 패키지의 일반 EFI를 실행한 결과는 아닙니다.

Native run 08 UART: 37,212 bytes, SHA-256 `30f2d22b8ed9ea44be585ef17166fce466740690e74822ea6e3bb3610ba08dd0`; run 09 UART: 183,269 bytes, SHA-256 `100b86919ef5d4d74bf08f035d10b1cefdac01d4339bbeb3f2d7107f16308233`. For these earlier runs the loader GOP remains text; run10 changes to the graphical page below. Run 09 WindowServer references are launchd job names/lint and failed-bootstrap logs, not proof that WindowServer runs. / 두 실행의 UART 크기와 해시를 확인했습니다. 이전 두 실행의 화면은 Apple 로더 텍스트에 머물렀으며 실행10의 그래픽 진전은 아래에 별도 기록했습니다. 실행 09의 WindowServer 문구는 작업 이름·검사 경고·등록 실패 기록이므로 해당 프로세스의 실행이나 GUI 표시를 입증하지 않습니다.

## Candidate files / 후보 파일

Clean package source / 깨끗한 패키지 소스: `c156d1f8085d5682262abbdc2263765a1de33dfa`; source-tree SHA-256 `38ae56c3f5b226dbb78ef5c64786eb15ffc1b19f32c3676111c7cc3362d0891b`. This package differs from the integration snapshot above in 11 Tools/crate paths; its artifacts retain their own older source identity. / 이 패키지는 위의 통합 소스와 도구·크레이트 파일 경로 11개가 다릅니다. 파일은 이전 소스 식별자를 그대로 유지합니다.

| Local candidate / 로컬 후보 | Bytes / 바이트 | SHA-256 |
| --- | ---: | --- |
| BOOTX64.EFI | 204288 | `c9f21d205cfa947d2dda142713023d5ed1f03dd8848d972a1eb8252bdc8461b8` |
| NXINSTALL.efi | 81408 | `f21ae995987f0f442808cbb2a4bd023da5977dffe340d3704e30b010103adb35` |
| NXARMJIT.efi | 425984 | `cd0cf6ebbbaa2e7251e6818bcce20303e52c082f44f97b6675dba0bb6c11a259` |
| Research installation ISO / 연구 설치 ISO | 18542592 | `65b3370858ad708ff8eb86e9f68fa21ea29bc2bb188528d88530c412c71ba1f7` |
| Complete ZIP / 전체 ZIP | 19625729 | `2326cc5a6512e7f090af850a1bc162ab4c356e561452728caf8891f540788ef2` |

The exact ISO boots the read-only utility, which verifies and starts its research EFI child. The child returns NOT_FOUND because private Apple inputs/configuration are excluded. The package does not reproduce the separate native userspace or ARM 26-instruction result. No new download URL or release is declared by this record. / 해당 ISO는 읽기 전용 유틸리티를 시작하고 연구용 자식 EFI의 해시를 검증한 뒤 실행합니다. 비공개 Apple 입력과 설정은 포함하지 않아 자식 EFI가 NOT_FOUND를 반환합니다. 별도의 네이티브 사용자 공간 또는 ARM 명령 26개 실행을 재현한 패키지는 아니며, 이 기록은 새 다운로드나 릴리즈를 선언하지 않습니다.

## Nine-repository publication / 관련 저장소 9개의 게시

`NextCore-Stuff`, `NextCore`, `Nextcore-Core`, `Nextcore-ISE`, `Nextcore-GPU`, `Nextcore-HAL`, `Nextcore-APLS`, `Nextcore-EFI`, `Nextcore-Tool`.

Publish reviewed Core/GPU/HAL/ISE revisions before dependent APLS/EFI and Tool, fix exact dependency pins, validate standalone/integrated source and one immutable matched artifact set, then publish a distinct preview and read back remote assets/hashes. Unchanged modules receive an explicit unchanged-source milestone record; no fictitious code change is claimed. Historical releases remain separately identified. / 검토한 Core/GPU/HAL/ISE 커밋을 먼저 게시하고 APLS/EFI·Tool의 정확한 의존성 핀을 맞춥니다. 독립·통합 소스와 단일 고정 파일 묶음을 검증한 뒤 별도 프리뷰를 게시하고 원격 파일과 해시를 다시 확인합니다. 변경 없는 모듈은 같은 소스의 마일스톤 기록으로 표시하며 코드 변경을 꾸미지 않습니다. 과거 릴리즈는 별도로 보존합니다.

**Acceptance / 제품 판정:** macOS 27 boot=false; Recovery GUI=false; installed GUI=false; Metal=false; physical installation/qualification=false. Native launchd main and Recovery userspace are observed partial progress, not completion of the full goal. / macOS 27 부팅, 두 OS GUI, Metal, 실제 설치와 기기 검증은 모두 false입니다. 네이티브 launchd main과 리커버리 사용자 공간은 관찰된 부분 진전이며 전체 목표의 완료를 뜻하지 않습니다.

OPEN_QUESTION: Complete native kernel-driver and ARM entry/mapping dependencies, verify interactive GUI stages, and finish source/pin plus binary preview publication across all nine repositories. / 네이티브 커널 드라이버와 ARM 진입·매핑 문제를 해결하고, 조작 가능한 GUI를 검증하며, 저장소 9개의 소스·핀과 바이너리 프리뷰 게시를 완료합니다.


## Graphical checkpoint / 그래픽 확인 — 2026-10-02 01:29:34 KST

Original native Tahoe26 run10 reaches actual WindowServer[80], RecoveryOS Agent[125] and an Apple keyboard/mouse pairing screen in Recovery/BaseSystem, without a Linux guest intermediary. CPU model94 + invariant TSC,3072MiB RAM and the original CoreServices booter preserve baseline EFI/source59 and original Recovery inputs. UART444,380 bytes SHA-256 `6448ffb026884a3cf85ec10ceb157c6637f8ec54e28ea1875f9e2ca1c50a6ac8`; visually inspected1280×800 PNG SHA-256 `91eb8f19dfeda0002c841aebf9007740ec5e36ef15f082ec90a7a8c41876854f`. Original inputs remain unchanged and the bounded guest is reaped. / Linux 없는 원본 네이티브 실행10에서 실제 WindowServer·RecoveryOS Agent·Apple 키보드/마우스 연결 안내 그래픽 화면을 확인했습니다. 기준 소스59·EFI·원본 Recovery를 유지하며 CPU94·불변 TSC·3072MiB 메모리를 사용했습니다. UART·실제 화면 해시를 별도 확인했고 원본 입력 보존·제한 시간 종료 후 게스트 정리를 확인했습니다.

Native target match passes the Tahoe dual gate (Darwin25/product26); **input response, usable Recovery utilities, installed GUI, Metal and ARM/macOS27 qualification remain unverified**. A visible pairing page is partial graphics progress. All nine repositories now have reviewable progress drafts; implementation source/pins, main integration and a new source-bound binary release remain pending. The c156 artifacts above retain their original older source identity. / 네이티브 Darwin25·제품26 대상 일치는 통과했으나 **입력 반응·복구 메뉴 사용·설치 GUI·Metal·ARM/macOS27 제품 검증은 미완료**입니다. 연결 안내 화면은 부분 그래픽 진전입니다. 9개 저장소 진행 초안은 공개됐으며 구현 소스/핀·main 통합·새 소스 연결 바이너리 릴리즈는 대기 중입니다. 위 c156 파일은 이전 소스 식별자를 유지합니다.

## Clean research EFI build / 깨끗한 연구 EFI 빌드

Clean source `98e117d53c81c7da86406bc0726c8ed2c7b6842a` now produces NXARMJIT.efi,428,544 bytes, SHA-256 `ad00dac79213c3ebe789f5688d888f40bf654c1cc4eb7346c187d8e686838d30`; source preservation, build exit0 and cleanup are verified. Its19 authored host cache cases pass with actual memory permissions. No original macOS input or guest is used in those cases, and no new original-input EFI guest result is verified. The source91 linker failure remains a separate historical attempt. / 깨끗한 소스98의 연구 EFI428,544바이트 생성·정확한 해시·소스 보존·빌드 종료0·정리를 확인했습니다. 자체 호스트 캐시 검사19개는 실제 권한 변경으로 통과했으며 원본 macOS 입력·게스트를 사용하지 않았습니다. 새 EFI 원본 입력 게스트 결과는 미검증이고 이전 소스91 연결 오류는 별도 이력으로 보존합니다.

This new research EFI is separate from the older source-c156 installer/ISO/ZIP above and is not a new published binary release. / 새 연구 EFI는 위의 이전 소스c156 인스톨러·ISO·ZIP과 별개이며 새 공개 바이너리 릴리즈는 아닙니다.
