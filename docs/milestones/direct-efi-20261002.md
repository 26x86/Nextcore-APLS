# NextCore direct EFI milestone / NextCore 직접 EFI 마일스톤

Current Status / 현재 상태: Original native Tahoe 26 reaches launchd main and Recovery/BaseSystem userspace without a Linux guest intermediary. A later run starts Recovery services and stops at a kernel general-protection trap. Both OS GUIs and macOS 27 product qualification remain unverified. / Linux 게스트를 거치지 않고 원본 네이티브 Tahoe 26에서 launchd main과 리커버리 사용자 공간에 진입했습니다. 후속 실행은 리커버리 서비스를 시작한 뒤 커널 일반 보호 오류에서 중단됐습니다. 두 OS GUI와 macOS 27 제품 검증은 아직 완료되지 않았습니다.

Target State / 목표 상태: Direct standard EFI → target-matching LaunchD → interactive Recovery GUI and installed GUI. Update all nine related repositories and provide one source-bound installer, ISO and EFI preview at every milestone. / 표준 EFI에서 대상에 맞는 LaunchD, 조작 가능한 리커버리 GUI와 설치 GUI까지 직접 도달하는 것이 목표입니다. 매 마일스톤마다 관련 저장소 9개를 갱신하고 동일 소스의 인스톨러·ISO·EFI 프리뷰를 제공합니다.

Snapshot / 확인 시각: 2026-10-02T01:03:52+09:00 KST. This is a draft progress record for review, not a live execution view. / 검토용 진행 기록이며 실시간 실행 화면이 아닙니다.

## This repository / 이 저장소

- Repository / 저장소: `26x86/Nextcore-APLS`.
- Role / 역할: Independently versioned source module / 독립 버전 소스 모듈.
- Remote main before this record / 기록 전 원격 main: `b8d29eb92fbbc0d369d09caedf61800bf44350a4`.
- Integrated source snapshot / 통합 소스 확인 버전: `69c5988cffc596773a3d70b3f006d9973d7f1444` in `26x86/NextCore-Stuff`.

The committed integration branch has no changed paths in this crate relative to the NextCore-Stuff base 59e41262; this documentation record does not alter its source or dependency pins. / 통합 브랜치에서 NextCore-Stuff 기준 커밋 59e41262 대비 이 크레이트의 변경 파일은 없습니다. 이 진행 기록은 소스나 의존성 핀을 바꾸지 않습니다.

## Reached stages / 도달한 단계

| Track / 경로 | Verified observation / 확인된 관찰 | Limit / 한계 |
| --- | --- | --- |
| Original native Tahoe 26 / 원본 네이티브 Tahoe 26 | Run 08 launchd main, BaseSystem userspace and Recovery context; run 09 Recovery services after RAM increase / 실행 08에서 launchd main·사용자 공간·리커버리 문맥 확인, 실행 09에서 메모리 증가 후 리커버리 서비스 시작 | Run 09 kernel trap; no interactive OS GUI / 실행 09의 커널 오류로 중단, 조작 가능한 OS GUI 미검증 |
| Original ARM/macOS 27 / 원본 ARM/macOS 27 | Clean source 6bff94d 26-instruction EFI prefix validated per invocation / 깨끗한 소스 6bff94d에서 EFI 명령 26개 구간을 실행별로 검증 | MPIDR stop; authored DRAM/software platform, no userspace / MPIDR에서 중단. 자체 DRAM·소프트웨어 플랫폼이며 사용자 공간은 미확인 |
| Authored firmware configuration / 자체 펌웨어 설정 | Configuration 15/picker 6 fixture cases pass; packaged normal EFI additionally passes case 2 / 설정 15개·선택기 6개 사례 통과, 패키지의 일반 EFI도 사례 2 통과 | Firmware UI is separate from macOS Recovery / 펌웨어 UI이며 macOS 리커버리와 구분 |
| Matched local research bundle / 동일 소스 로컬 연구 묶음 | Three EFIs, ISO and ZIP structurally verified at clean c156d1f / 깨끗한 c156d1f에서 EFI 3개·ISO·ZIP 구조 검증 | New remote release and qualified installation pending / 새 원격 릴리즈와 제품 설치 검증은 대기 중 |

Later ARM EFI observations without a valid trace return are separate and do not supersede the validated source-6bff94d 26-instruction prefix. / 이후의 ARM EFI 관찰에서 유효한 추적 종료가 없었던 결과는 별도 증거이며, 소스 6bff94d에서 검증한 명령 26개 구간을 대체하지 않습니다.

Native runs use original Tahoe 26.6.2 (build 25G83), Darwin 25.6 and normal EFI from baseline source 59e41262, SHA-256 `acaecadd7e9d3ebec9699fb0ac57d7dc15bc4584a38db56d878e1cd41bca0e64`. Original inputs were preserved, backing remained read-only, writes used a private overlay and owned guests were reaped. They do not execute the newer packaged normal EFI. / 네이티브 실행은 원본 Tahoe 26.6.2(build 25G83)·Darwin 25.6과 기준 소스 59e41262의 일반 EFI를 사용했습니다. 원본 입력 보존, 읽기 전용 원본, 비공개 쓰기 오버레이와 게스트 정리를 확인했습니다. 새 패키지의 일반 EFI를 실행한 결과는 아닙니다.

Native run 08 UART: 37,212 bytes, SHA-256 `30f2d22b8ed9ea44be585ef17166fce466740690e74822ea6e3bb3610ba08dd0`; run 09 UART: 183,269 bytes, SHA-256 `100b86919ef5d4d74bf08f035d10b1cefdac01d4339bbeb3f2d7107f16308233`. The loader GOP remains text; later stages are established by UART. Run 09 WindowServer references are launchd job names/lint and failed-bootstrap logs, not proof that WindowServer runs. / 두 실행의 UART 크기와 해시를 확인했습니다. 화면은 Apple 로더 텍스트에 머물렀으며 후속 단계는 UART로 확인했습니다. 실행 09의 WindowServer 문구는 작업 이름·검사 경고·등록 실패 기록이므로 해당 프로세스의 실행이나 GUI 표시를 입증하지 않습니다.

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
