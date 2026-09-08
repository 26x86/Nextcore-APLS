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

The [SGPU grant adapter](SGPU_GRANT_TRANSPORT.md) connects the existing inline
frame format and VSK's 64-byte granted message to GPU's bounded compute session.
Root-owned caller identity, canonical grant references and endpoint authorization
remain explicit caller responsibilities. Default builds have no Vulkan backend;
`--features vulkan` enables only the host development acceptance example.

The RX 6800 XT [grant-path receipt](validation/sgpu-grant-rx6800xt-20260908/README.md)
records four actual GPU dispatches and 512 independently checked readback values.
This is authored transport/backend validation, not actual VSK-root or guest-driver
submission, physical EFI GPU execution, or macOS Metal.
