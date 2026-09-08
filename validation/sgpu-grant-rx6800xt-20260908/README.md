# VSK grant transport on AMD Radeon RX 6800 XT

The authored example ran natively on Windows with PCI 1002:73bf, driver
32.0.21043.10005. Its existing APLS inline payload was converted into a VSK
64-byte header and a separate immutable grant snapshot, then authorized, decoded
by the shared GPU codec, and executed by SgpuComputeSession.

Two submitted lists completed four real Vulkan compute dispatches and two
software buffer copies. All 512 raw output words match an independent Python
recomputation of two successive `value * 3 + 7` transforms. Binding guards stayed
unchanged; one native pipeline compilation served all four dispatches. Nine
invalid grant/route/lifetime/range cases preserved data and completion count.

The module compiled independently against published immutable GPU revision
34d617cca37d563d13ed0b0c966e2557921c6125. Source hashes in receipt.json were compared
against the exact standalone tree used to build the Windows executable. Raw
readback is retained in hardware-example.json. Default and optional-Vulkan Rust
suite logs and the standalone dependency lock accompany this receipt. The
unchanged public VSK C policy matched Rust on 892 cases / 1,784 signed-status
comparisons under UBSan, including short/trailing header lengths and explicit
family/opcode boundaries. The partial-backend-failure test verifies that only an
already completed prefix survives, with later work stopped.

Reproduction commands and root-lock/reference prerequisites are documented in
../../SGPU_GRANT_TRANSPORT.md. Use the Vulkan 1.1 shader compiler target shown
there; the backend accepts StorageBuffer SPIR-V, not legacy Uniform storage.

The caller/grant metadata is an authored fixture. No actual VSK root dispatcher,
guest driver, physical EFI GPU command backend, macOS boot or guest Metal was
executed. WSL only builds and runs CPU policy tests; the hardware dispatch ran
through the Windows Vulkan driver. No CPU fallback supplied the compute output.
