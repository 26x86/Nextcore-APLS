# Nextcore-APLS

AArch64 recovery diagnostics and public execution interfaces. The production
architecture is macOS ARM64e translated on an x86 computer at EFI startup;
external recovery fixtures remain diagnostic tools.

This independent repository consumes Nextcore-GPU through the immutable Git
revision in `Cargo.toml`. In the integration repository, Cargo patches that URL
to the matching GPU submodule. No sibling source checkout is needed for a
standalone build:

```sh
cargo test --all-targets
```

Previous release provenance is preserved in `repository.json`. Public source
only; runtime acceptance of macOS boot and guest Metal remains unfinished.
