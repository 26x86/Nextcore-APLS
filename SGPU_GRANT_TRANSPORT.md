# SGPU VSK grant adapter — BP27 contract

Current: APLS inline frames contain a 64-byte header plus a bounded body. VSK
vf_msg_header carries exactly 64 message bytes and refers to a separate grant.
VSK provides public header/grant policy but no executable SGPU router. GPU owns
the only SGPU command codec and its validated synchronous compute session.

Decision: add an APLS adapter using those existing formats. The exact C policy
in sandbox/vsk/include/{vf_abi,vf_policy}.h and src/vf_policy.c is the authority
for header shape, caller-scoped generation:slot handles, rights, validity window,
canonical grant lifecycle, duplicate-slot rejection and payload bounds. Portable
Rust views mirror those checks; optional differential tests compile the unchanged
C implementation from an explicitly supplied checkout. No C sources are copied
into this module and no new guest wire protocol is introduced.

The root supplies trusted, refreshed metadata and an immutable snapshot window
of the same canonical grant object. Borrowed slices are not physical addresses.
The root must hold its table lock from authorization through reference acquisition
or this synchronous submission, and retain the canonical reference until return.
Snapshots must be copied while stable or otherwise protected against guest writes;
a Rust slice alone does not freeze shared guest memory. Root-owned cell identity,
time and route are never taken from guest payloads. A session binds one caller cell
and one endpoint handle already authorized by the root; header.object must match.
This adapter does not invent an endpoint-rights convention or authenticate root
inputs, acquire references, map pages, revoke grants, or isolate a service cell.
Duplicated capabilities refer to one lifecycle; stale copied metadata is invalid
caller usage. Every submission rechecks current generation, state and validity.

Inline-to-grant conversion returns a header plus a borrowed existing payload;
it neither publishes nor authorizes a grant. Authorized grant-to-inline conversion
normalizes payload offset and clears the inline grant handle. Such a frame is only
a data container and never carries authority into a later submission. Direct
submission authorizes READ, takes the checked snapshot window, and reuses GPU's
bounded decoder and SgpuComputeSession. Only SUBMIT_COMMAND_LIST is supported by
this compute adapter; known unimplemented opcodes fail. Entire-list preflight and
resource/kernel lifetime rules remain GPU-owned. Backend failure can retain a
completed prefix and returns its count; no rollback or invented success occurs.

Validation: unit tests exercise policy, snapshot ranges, revocation, routing,
conversion and ordered software copy. A separately selected Vulkan example may
exercise the same adapter on the physical RX 6800 XT with authored commands and
independent readback. Vulkan is optional development validation only. This does
not establish macOS boot, guest-driver submission, guest Metal, or an EFI GPU
command backend. Those outcomes remain false.

OPEN_QUESTION: None within this adapter contract. Executable trusted-root routing,
canonical reference management and a physical EFI GPU backend are separate work.

## Reproduction and remaining product integration

```sh
cargo test --all-targets
NEXTCORE_VSK_POLICY_ROOT=/path/to/26x86/sandbox/vsk \
  cargo test --test sgpu_grant existing_vsk_c_policy_differential -- --ignored --nocapture
glslangValidator -V --target-env vulkan1.1 examples/sgpu_grant.comp -o sgpu_grant.spv
cargo run --features vulkan --example sgpu_grant_vulkan -- \
  1002 73bf /path/to/spirv-val /path/to/sgpu_grant.spv
```

Use the actual host OS Vulkan driver for the last command. WSL's software ICD
is not proof of the physical Windows adapter. The shader uses the existing
backend's SPIR-V StorageBuffer profile; compile with the explicit Vulkan 1.1
target above. Exact output and tool/source hashes are in the public validation
receipt linked from README. The C oracle covers 892 input cases, comparing
both decode and payload-policy signed statuses, with UBSan enabled.

The next narrow product step is an actual root dispatcher that routes this
existing 64-byte message, authorizes the endpoint using its canonical capability
table, acquires the grant reference under the root lock, freezes/copies the
selected payload, and returns explicit completion/error information. This must
use real lifecycle transitions and cannot take fixture metadata as authority.
The physical EFI path then needs a selected PCI GPU's initialization, bounded
command/ring submission, mapped resource ownership and real completion/readback;
GOP scanout alone supplies none of these operations. The current Vulkan backend
cannot be linked into EFI as an OS-library shortcut. Finally an exact guest
adapter must produce these submissions through the guest's real graphics API
and return observable Metal compute/render results. Creating a synthetic guest
driver or a successful host receipt cannot satisfy that final boundary.
