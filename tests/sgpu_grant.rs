use nextcore_apls::{
    sgpu::{SgpuCommand, SgpuCommandList, SgpuFrame},
    sgpu_grant::*,
    vf_abi::*,
};
use nextcore_gpu::{compute::ComputePipelineManager, sgpu_compute::SgpuComputeSession};

const CELL: u32 = 42;
const ENDPOINT: u64 = 0x1122_3344_0000_0005;
const HANDLE: u64 = 0xaabb_ccdd_0000_0007;
fn grant() -> VfGrantEntry {
    VfGrantEntry {
        cap: VfCapEntry {
            slot: 7,
            generation: 0xaabb_ccdd,
            holder_cell: CELL,
            object_type: VF_OBJECT_GRANT,
            rights: VF_RIGHT_READ,
            state: VF_CAP_LIVE,
            object_id: 0x9988_7766_5544_3322,
            valid_from: 10,
            expires_at: 20,
        },
        bytes: 1024,
        owner_cell: 11,
        state: VF_GRANT_LIVE,
    }
}
fn header(len: usize) -> VfMsgHeader {
    let mut h = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_SUBMIT_COMMAND_LIST);
    h.request_id = 0xfedc_ba98_7654_3210;
    h.object = ENDPOINT;
    h.payload_grant = HANDLE;
    h.payload_offset = 8;
    h.payload_bytes = len as u32;
    h
}
fn session() -> SgpuGrantSession {
    SgpuGrantSession::new(
        SgpuComputeSession::new(ComputePipelineManager::new()),
        CELL,
        ENDPOINT,
    )
    .unwrap()
}
fn policy(h: &VfMsgHeader, entries: &[VfGrantEntry], caller: u32, rights: u32, now: u64) -> i32 {
    let views: Vec<_> = entries
        .iter()
        .map(|entry| VfGrantSnapshot {
            entry,
            snapshot_offset: 0,
            data: &[],
        })
        .collect();
    validate_payload_grant(h, &views, caller, rights, now)
        .map(|_| 0)
        .unwrap_or_else(|e| e as i32)
}

#[test]
fn strict_header_profile_and_copy_in() {
    let h = header(4);
    let mut unaligned = vec![99];
    unaligned.extend(h.encode());
    assert_eq!(decode_grant_header(&unaligned[1..]).unwrap(), h);
    for size in 0..64 {
        assert_eq!(
            decode_grant_header(&unaligned[..size]),
            Err(GrantError::Eabi)
        );
    }
    assert_eq!(decode_grant_header(&unaligned), Err(GrantError::Eabi));
    for (h, expected) in [
        (
            VfMsgHeader {
                message_bytes: 68,
                ..h
            },
            GrantError::Eabi,
        ),
        (VfMsgHeader { flags: 1, ..h }, GrantError::Einval),
        (VfMsgHeader { reserved0: 1, ..h }, GrantError::Eabi),
        (VfMsgHeader { opcode: 8, ..h }, GrantError::Enotsup),
        (
            VfMsgHeader {
                payload_offset: 1,
                ..h
            },
            GrantError::Einval,
        ),
        (
            VfMsgHeader {
                payload_offset: u64::MAX - 7,
                payload_bytes: 8,
                ..h
            },
            GrantError::Erange,
        ),
        (
            VfMsgHeader {
                payload_bytes: VF_PAYLOAD_MAX + 1,
                ..h
            },
            GrantError::Erange,
        ),
        (
            VfMsgHeader {
                payload_bytes: 0,
                ..h
            },
            GrantError::Einval,
        ),
    ] {
        assert_eq!(validate_grant_header(&h), Err(expected));
    }
}

#[test]
fn policy_generation_rights_validity_duplicates_and_range() {
    let h = header(4);
    let g = grant();
    assert_eq!(policy(&h, &[g], CELL, VF_RIGHT_READ, 10), 0);
    for now in [0, 9, 20, u64::MAX] {
        assert_eq!(policy(&h, &[g], CELL, VF_RIGHT_READ, now), -4);
    }
    assert_eq!(policy(&h, &[g], CELL, VF_RIGHT_WRITE, 15), -5);
    assert_eq!(policy(&h, &[g], CELL + 1, VF_RIGHT_READ, 15), -4);
    assert_eq!(policy(&h, &[g, g], CELL, VF_RIGHT_READ, 15), -9);
    assert_eq!(
        policy(
            &h,
            &vec![g; VF_POLICY_TABLE_MAX + 1],
            CELL,
            VF_RIGHT_READ,
            15
        ),
        -1
    );
    assert_eq!(policy(&h, &[g], 0, VF_RIGHT_READ, 15), -1);
    for rights in [0, 4, 8, 255] {
        assert_eq!(policy(&h, &[g], CELL, rights, 15), -1);
    }
    for (field, want) in [
        (0, -4),
        (1, -4),
        (2, -5),
        (3, -5),
        (4, -4),
        (5, -4),
        (6, -6),
    ] {
        let mut changed = g;
        match field {
            0 => changed.cap.generation += 1,
            1 => changed.cap.state = 2,
            2 => changed.cap.object_id = 0,
            3 => changed.cap.rights |= 256,
            4 => changed.state = 2,
            5 => changed.owner_cell = 0,
            _ => changed.bytes = 11,
        }
        assert_eq!(policy(&h, &[changed], CELL, VF_RIGHT_READ, 15), want);
    }
    // Owner and holder are distinct; delegated READ is valid without owner==caller.
    assert_ne!(g.owner_cell, CELL);
}

#[test]
fn inline_grant_conversion_normalizes_only_data_and_rechecks_revocation() {
    let payload = 0u32.to_le_bytes().to_vec();
    let mut inline_header = header(4);
    inline_header.message_bytes = 76;
    let frame = SgpuFrame {
        header: inline_header,
        payload,
    };
    let (wire, bytes) = inline_to_grant(&frame, HANDLE, 8).unwrap();
    let mut g = grant();
    let session = session();
    {
        let views = [VfGrantSnapshot {
            entry: &g,
            snapshot_offset: 8,
            data: bytes,
        }];
        let normalized = session.to_inline(&wire, &views, CELL, 15).unwrap();
        assert_eq!(normalized.payload, frame.payload);
        assert_eq!(normalized.header.message_bytes, 68);
        assert_eq!(normalized.header.payload_offset, 0);
        assert_eq!(normalized.header.payload_grant, 0);
        assert_eq!(normalized.header.request_id, frame.header.request_id);
        assert_eq!(
            SgpuFrame::from_wire(&normalized.try_to_wire().unwrap()).unwrap(),
            normalized
        );
        assert!(session
            .to_inline(&normalized.header.encode(), &views, CELL, 15)
            .is_err());
    }
    g.state = 2;
    g.cap.state = 2;
    let views = [VfGrantSnapshot {
        entry: &g,
        snapshot_offset: 8,
        data: bytes,
    }];
    assert!(matches!(
        session.to_inline(&wire, &views, CELL, 15),
        Err(GrantSubmitError::Grant(GrantError::Estale))
    ));
}

#[test]
fn snapshot_ranges_route_and_supported_opcode_are_checked_before_execution() {
    let bytes = [0, 0, 0, 0];
    let g = grant();
    let h = header(4);
    let mut session = session();
    for (offset, data) in [
        (9, bytes.as_slice()),
        (8, &bytes[..3]),
        (u64::MAX, bytes.as_slice()),
        (1022, bytes.as_slice()),
    ] {
        let views = [VfGrantSnapshot {
            entry: &g,
            snapshot_offset: offset,
            data,
        }];
        assert!(matches!(
            session.submit(&h.encode(), &views, CELL, 15),
            Err(GrantSubmitError::Grant(GrantError::Erange))
        ));
    }
    let views = [VfGrantSnapshot {
        entry: &g,
        snapshot_offset: 8,
        data: &bytes,
    }];
    assert!(matches!(
        session.submit(&h.encode(), &views, CELL + 1, 15),
        Err(GrantSubmitError::Grant(GrantError::Eperm))
    ));
    let wrong = VfMsgHeader {
        object: ENDPOINT + 1,
        ..h
    };
    assert!(matches!(
        session.submit(&wrong.encode(), &views, CELL, 15),
        Err(GrantSubmitError::Grant(GrantError::Eperm))
    ));
    for opcode in [1, 2, 3, 4, 6, 7] {
        let wrong = VfMsgHeader { opcode, ..h };
        assert!(matches!(
            session.submit(&wrong.encode(), &views, CELL, 15),
            Err(GrantSubmitError::Grant(GrantError::Enotsup))
        ));
    }
    assert_eq!(session.submit(&h.encode(), &views, CELL, 15).unwrap(), 0);
}

#[test]
fn grant_to_existing_executor_copy_and_atomic_preflight() {
    let mut manager = ComputePipelineManager::new();
    let src = manager.create_storage_buffer(16);
    let dst = manager.create_storage_buffer(16);
    let mut compute = SgpuComputeSession::new(manager);
    compute
        .register_resource(0xfeed_beef_0000_0001, src)
        .unwrap();
    compute
        .register_resource(0xabcd_ef00_0000_0002, dst)
        .unwrap();
    compute
        .write_resource(0xfeed_beef_0000_0001, 0, &[0xa5; 16])
        .unwrap();
    let copy = SgpuCommand::CopyBuffer {
        src: 0xfeed_beef_0000_0001,
        dst: 0xabcd_ef00_0000_0002,
        size: 16,
    };
    let mut session = SgpuGrantSession::new(compute, CELL, ENDPOINT).unwrap();
    let g = grant();
    for unsupported in [true, false] {
        let mut cmds = vec![copy.clone()];
        if unsupported {
            cmds.push(SgpuCommand::Present {
                buffer: 0xabcd_ef00_0000_0002,
            });
        }
        let bytes = SgpuCommandList { cmds }.encode();
        let h = header(bytes.len());
        let views = [VfGrantSnapshot {
            entry: &g,
            snapshot_offset: 8,
            data: &bytes,
        }];
        let result = session.submit(&h.encode(), &views, CELL, 15);
        if unsupported {
            assert!(matches!(result, Err(GrantSubmitError::Submission(ref e)) if e.completed == 0));
            assert_eq!(
                session
                    .compute()
                    .read_resource(0xabcd_ef00_0000_0002, 0, 16)
                    .unwrap(),
                [0; 16]
            );
        } else {
            assert_eq!(result.unwrap(), 1);
            assert_eq!(
                session
                    .compute()
                    .read_resource(0xabcd_ef00_0000_0002, 0, 16)
                    .unwrap(),
                [0xa5; 16]
            );
        }
    }
    assert_eq!(session.compute().completed_command_count(), 1);
}

/// Reproducible cross-implementation check, not a substitute VSK implementation.
/// NEXTCORE_VSK_POLICY_ROOT points to sandbox/vsk in an existing public checkout.
#[test]
#[ignore = "requires C compiler and explicit NEXTCORE_VSK_POLICY_ROOT"]
fn existing_vsk_c_policy_differential() {
    use std::{
        fmt::Write as _,
        io::Write as _,
        process::{Command, Stdio},
    };
    let root = std::path::PathBuf::from(
        std::env::var_os("NEXTCORE_VSK_POLICY_ROOT").expect("set NEXTCORE_VSK_POLICY_ROOT"),
    );
    let temporary =
        std::env::temp_dir().join(format!("nextcore-vsk-policy-{}", std::process::id()));
    std::fs::create_dir_all(&temporary).unwrap();
    let executable = temporary.join("oracle");
    let build = Command::new("cc")
        .args([
            "-std=c17",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-O1",
            "-fsanitize=undefined",
        ])
        .arg("-I")
        .arg(root.join("include"))
        .arg(root.join("src/vf_policy.c"))
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vsk_policy_oracle.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let h = header(4);
    let g = grant();
    let mut cases = vec![(h, vec![g], CELL, VF_RIGHT_READ, 15)];
    // Every header bit plus independently chosen multibyte boundaries.
    for byte in 0..64 {
        for bit in 0..8 {
            let mut raw = h.encode();
            raw[byte] ^= 1 << bit;
            cases.push((
                VfMsgHeader::decode(&raw).unwrap(),
                vec![g],
                CELL,
                VF_RIGHT_READ,
                15,
            ));
        }
    }
    for rights in [0, 1, 2, 3, 4, 255, 256, u32::MAX] {
        for caller in [0, CELL, CELL + 1] {
            for now in [0, 9, 10, 19, 20, u64::MAX] {
                cases.push((h, vec![g], caller, rights, now));
            }
        }
        let mut changed = g;
        changed.cap.rights = rights;
        cases.push((h, vec![changed], CELL, VF_RIGHT_READ, 15));
    }
    for value in [0, 1, 2, 7, 42, u32::MAX] {
        for field in 0..8 {
            let mut changed = g;
            match field {
                0 => changed.cap.slot = value,
                1 => changed.cap.generation = value,
                2 => changed.cap.holder_cell = value,
                3 => changed.cap.object_type = value,
                4 => changed.cap.state = value,
                5 => changed.owner_cell = value,
                6 => changed.state = value,
                _ => changed.cap.rights = value,
            }
            cases.push((h, vec![changed], CELL, VF_RIGHT_READ, 15));
        }
    }
    for value in [0, 1, 7, 8, 11, 12, 15, 20, u64::MAX] {
        for field in 0..4 {
            let mut changed = g;
            match field {
                0 => changed.cap.object_id = value,
                1 => changed.cap.valid_from = value,
                2 => changed.cap.expires_at = value,
                _ => changed.bytes = value,
            }
            cases.push((h, vec![changed], CELL, VF_RIGHT_READ, 15));
        }
    }
    cases.push((h, vec![], CELL, VF_RIGHT_READ, 15));
    cases.push((h, vec![g, g], CELL, VF_RIGHT_READ, 15));
    let mut other = g;
    other.cap.holder_cell += 1;
    cases.push((h, vec![other, g], CELL, VF_RIGHT_READ, 15));
    for offset in [0, 1, 8, u64::MAX - 7, u64::MAX] {
        for length in [0, 4, VF_PAYLOAD_MAX, VF_PAYLOAD_MAX + 1] {
            cases.push((
                VfMsgHeader {
                    payload_offset: offset,
                    payload_bytes: length,
                    ..h
                },
                vec![g],
                CELL,
                1,
                15,
            ));
        }
    }
    for family in [0, 1, 2, 3, 4, u16::MAX] {
        for opcode in [0, 1, 3, 4, 7, 8, 9, 10, u16::MAX] {
            cases.push((
                VfMsgHeader {
                    family,
                    opcode,
                    ..h
                },
                vec![g],
                CELL,
                VF_RIGHT_READ,
                15,
            ));
        }
    }
    let mut input = String::new();
    let mut expected = Vec::new();
    for (h, entries, caller, rights, now) in &cases {
        for byte in h.encode() {
            write!(&mut input, "{byte:02x}").unwrap();
        }
        write!(&mut input, " 64 {caller} {rights} {now} {}", entries.len()).unwrap();
        for g in entries {
            let c = g.cap;
            write!(
                &mut input,
                " {} {} {} {} {} {} {} {} {} {} {} {}",
                c.slot,
                c.generation,
                c.holder_cell,
                c.object_type,
                c.rights,
                c.state,
                c.object_id,
                c.valid_from,
                c.expires_at,
                g.bytes,
                g.owner_cell,
                g.state
            )
            .unwrap();
        }
        input.push('\n');
        expected.push((
            decode_grant_header(&h.encode())
                .map(|_| 0)
                .unwrap_or_else(|e| e as i32),
            policy(h, entries, *caller, *rights, *now),
        ));
    }
    let template = input.lines().next().unwrap().to_owned();
    for count in (0..64u32).chain([65, u32::MAX]) {
        input.push_str(&template.replacen(" 64 ", &format!(" {count} "), 1));
        input.push('\n');
        expected.push((-2, 0));
    }
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "C undefined-behavior diagnostic: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let actual: Vec<_> = text
        .lines()
        .map(|line| {
            let values: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            assert_eq!(values.len(), 2);
            (values[0], values[1])
        })
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            actual,
            expected,
            "C/Rust policy differs at case {index}: {:?}",
            cases.get(index)
        );
    }
    println!(
        "unchanged VSK C/Rust differential cases: {} (header + grant statuses), UBSan clean",
        expected.len()
    );
}

#[test]
fn backend_failure_retains_only_completed_prefix_through_grant_adapter() {
    use nextcore_gpu::compute::{
        BufferBinding, ComputeError, ComputePipelineDescriptor, ComputeShader,
    };
    use nextcore_gpu::sgpu_compute::SgpuComputeError;
    let mut manager = ComputePipelineManager::new();
    let source = manager.create_storage_buffer(16);
    let destination = manager.create_storage_buffer(16);
    let later = manager.create_storage_buffer(16);
    let pipeline = manager.create_pipeline(ComputePipelineDescriptor {
        shader: ComputeShader {
            id: 1,
            entry_point: "main".into(),
            bytecode: vec![],
        },
        workgroup_size: [1, 1, 1],
        buffer_bindings: vec![BufferBinding {
            binding: 0,
            buffer_id: destination,
            offset: 0,
            size: 16,
        }],
    });
    let mut compute = SgpuComputeSession::new(manager);
    for (guest, buffer) in [(1, source), (2, destination), (3, later)] {
        compute.register_resource(guest, buffer).unwrap();
    }
    compute.register_kernel(7, pipeline).unwrap();
    compute.write_resource(1, 0, &[0xa5; 16]).unwrap();
    let payload = SgpuCommandList {
        cmds: vec![
            SgpuCommand::CopyBuffer {
                src: 1,
                dst: 2,
                size: 16,
            },
            SgpuCommand::ComputeDispatch {
                kernel_id: 7,
                grid: (1, 1, 1),
                block: (1, 1, 1),
            },
            SgpuCommand::CopyBuffer {
                src: 1,
                dst: 3,
                size: 16,
            },
        ],
    }
    .encode();
    let mut session = SgpuGrantSession::new(compute, CELL, ENDPOINT).unwrap();
    let g = grant();
    let views = [VfGrantSnapshot {
        entry: &g,
        snapshot_offset: 8,
        data: &payload,
    }];
    let error = session
        .submit(&header(payload.len()).encode(), &views, CELL, 15)
        .unwrap_err();
    assert!(
        matches!(error,GrantSubmitError::Submission(ref e) if e.completed==1 && e.command_index==Some(1)
        && matches!(e.error,SgpuComputeError::Executor(nextcore_gpu::command_executor::ExecutorError::Compute(ComputeError::BackendUnavailable))))
    );
    assert_eq!(
        session.compute().read_resource(2, 0, 16).unwrap(),
        [0xa5; 16]
    );
    assert_eq!(session.compute().read_resource(3, 0, 16).unwrap(), [0; 16]);
    assert_eq!(session.compute().completed_command_count(), 1);
}
