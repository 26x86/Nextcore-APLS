# SGPU inline frame and resource checks — BP26-B

Current defects: codec ownership moved to GPU, changing the public decode error
type; the APLS inline frame decoder accepts truncated or trailing transfer bytes;
and CREATE_RESOURCE reaches an infallible backing allocator with arbitrary u64
sizes. The desired state preserves existing command payloads and valid inline
frames while returning errors before invalid data can allocate or mutate state.

Decisions:

- Map GPU SgpuWireError to APLS SgpuError::Payload through From, retaining `?`
  propagation into the existing APLS Result alias. Direct returns from a function
  explicitly typed as `sgpu::Result<_>` need `.map_err(Into::into)` because the
  shared codec's own return type is now `Result<_, SgpuWireError>`.
- Validate the SGPU family and existing VF header shape, the declared full-message
  size against the exact input length, checked u64 payload offset/end arithmetic,
  usize conversions, and the payload's exact declared size. Bound the complete
  inline body (including padding) by existing VF_PAYLOAD_MAX = 16 MiB.
- Keep valid to_wire output compatible. Add fallible try_to_wire for unchecked
  caller-built frames; validate before reservation/copy. The legacy to_wire API
  is for trusted validated frames and panics on invalid input instead of
  silently constructing malformed data; untrusted callers use try_to_wire.
- Validate caller-built frames at dispatch too, before any resource mutation.
  QUERY_CAPS has an empty request payload; resource create/destroy/present each
  require exactly eight bytes. SUBMIT retains the shared bounded command codec.
- Reuse GPU's public registration limits for SGPU resource creation: 16 MiB per
  resource, 64 MiB total, and 64 live resources. Reject zero as Payload and
  oversized/nonrepresentable requests, exhausted counts/bytes, and handle
  exhaustion as ResourceLimit before entering the backing allocator. Resource
  removal restores the live budget. This does not change GPU allocator policy.

The APLS inline frame stores payload bytes after its 64-byte header. It is not
identical to VSK's direct root ABI, whose 64-byte message refers to a separately
validated grant-backed payload. Bridging those transport forms is separate work;
these checks do not authenticate callers/grants or establish guest Metal.

Regression checks cover old `?` propagation and explicit return conversion,
header/payload round trips with padding, all truncations, trailing bytes,
declared-length mismatches, offset overflow, invalid caller-built dispatches,
strict operation sizes, huge resource requests, independent count/byte budgets,
and reclaiming budget after destruction. GPU source and metadata remain frozen.

## Validation result

`cargo test -p nextcore-apls` passed 57 tests on WSL2, including nine new checks
above. A separate executable compiled against the public APLS/GPU rlibs verified
that the previously accepted truncated/trailing frames are rejected, the former
u64::MAX allocation panic now returns ResourceLimit without consuming either
handle counter, and both legacy `?` propagation and explicit direct-return
conversion compile and run. `git diff --check` passed. No GPU execution code,
allocator-global policy, dependency pin, or repository metadata was changed.
No macOS guest Metal execution was performed for this transport change.
