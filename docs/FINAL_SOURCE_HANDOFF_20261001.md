# Final public source handoff for nextcore-apls

## Current Status

This module retains its independent GitHub repository and current main API. The final handoff preserves frozen historical public variants from an older local checkout. These unfinished variants are stored with their original module-relative paths under `research/handoff/legacy-uncommitted-20261001/`. They preserve authored public work and license notices for later review in another local development environment.

The archive is reference material. Its Rust/C source, repository metadata and historical README files are not current implementation or execution evidence. Hash verification establishes exact source preservation only. No compiler, test, EFI, guest, macOS boot, Metal, or physical-device acceptance result is established by this handoff.

## Target State

Keep every available public revision in this module's own main ancestry and preserve each frozen public variant with SHA-256, Git blob identity, source revision, relative path, mode and license provenance. The module remains independently versioned; its history is not flattened into the parent repository. Future work starts from the public module main in a different local environment.

The current active source, build configuration, API and top-level license remain unchanged by this archive. Review historical variants against the current API before selecting any future implementation change. Retire the scoped working branches and development checkouts only after publication and remote source coverage are independently verified.

## Preservation record

- [Archive scope and provenance](../research/handoff/legacy-uncommitted-20261001/HANDOFF.md)
- [Exact source hash manifest](../research/handoff/legacy-uncommitted-20261001/manifest.json)
- [Frozen license notices](../research/handoff/legacy-uncommitted-20261001/LICENSE.txt)

## Local history coverage

The local handoff revision `549a47e0a3f4554099a573a67362bde5c7445c18` contains every observed legacy branch, cached remote ref, tag target and detached HEAD for this module. 0 commit identities absent from the initial authoritative main were preserved by ordinary merges where required. The current active source tree remains byte-identical to the initial authoritative main; changes are confined to these handoff documents and research archives. This establishes local source coverage. Remote publication and coverage must be independently verified before removing any checkout.

## Open questions

OPEN_QUESTION: Verification: Historical source variants require independent reconciliation and target-bound execution in the next authorized development environment before any capability claim.

## BP74 exported source snapshot

The 2026-09-24 export candidate is an independent generated root commit referencing public parent source `93e4a9ebc1a05e2cbec78125a0186ca1b71d98d6`. Its identity and exact original tree are preserved by an ordinary merge of the unrelated public history. Older variants that differ from the current module source are retained as inactive research under `research/handoff/bp74-export-20260924/`, with original paths, blob identifiers, SHA-256 hashes and license provenance. Current source APIs, dependency pins and execution behavior remain active. Historical export metadata makes no new execution or device acceptance claim. Remote publication must be verified separately.

- [Export snapshot handoff](../research/handoff/bp74-export-20260924/HANDOFF.md)
- [Original tree and exact variant manifest](../research/handoff/bp74-export-20260924/manifest.json)
