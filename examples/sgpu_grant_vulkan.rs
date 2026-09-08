//! Authored VSK grant -> shared SGPU compute -> physical host Vulkan readback.
//! Development validation only: no guest driver, EFI GPU backend, or Metal claim.
use nextcore_apls::{sgpu::SgpuFrame, sgpu_grant::*, vf_abi::*};
use nextcore_gpu::{
    compute::{BufferBinding, ComputePipelineDescriptor, ComputePipelineManager, ComputeShader},
    sgpu_compute::SgpuComputeSession,
    vulkan_compute::{VulkanComputeConfig, VulkanDeviceSelector},
};
use std::{error::Error, path::PathBuf, time::Duration};

const CELL: u32 = 42;
const ENDPOINT: u64 = 0x1122_3344_0000_0005;
const GRANT: u64 = 0xaabb_ccdd_0000_0007;
const SOURCE: u64 = 0x1234_5678_0000_0001;
const DESTINATION: u64 = 0xfedc_ba98_0000_0002;
const KERNEL: u32 = 0xf100_0001;
fn run() -> std::result::Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: sgpu_grant_vulkan <vendor-hex> <device-hex> <spirv-val-path> <authored-spv-path>".into());
    }
    let parse = |arg: &std::ffi::OsStr| -> std::result::Result<u32, Box<dyn Error>> {
        Ok(u32::from_str_radix(
            arg.to_str()
                .ok_or("non-UTF8 selector")?
                .trim_start_matches("0x"),
            16,
        )?)
    };
    if std::fs::metadata(&args[3])?.len() > 256 * 1024 {
        return Err("example shader exceeds 256 KiB".into());
    }
    let shader = std::fs::read(&args[3])?;
    let mut manager = ComputePipelineManager::with_vulkan(VulkanComputeConfig {
        selector: VulkanDeviceSelector {
            vendor_id: parse(&args[0])?,
            device_id: parse(&args[1])?,
        },
        spirv_validator: PathBuf::from(&args[2]),
        fence_timeout: Duration::from_secs(3),
    })?;
    let device = manager
        .vulkan_device_info()
        .ok_or("missing Vulkan device")?
        .clone();
    let source = manager.create_storage_buffer(1056);
    let destination = manager.create_storage_buffer(1056);
    let pipeline = manager.create_pipeline(ComputePipelineDescriptor {
        shader: ComputeShader {
            id: 1,
            entry_point: "main".into(),
            bytecode: shader,
        },
        workgroup_size: [64, 1, 1],
        buffer_bindings: vec![BufferBinding {
            binding: 0,
            buffer_id: destination,
            offset: 16,
            size: 1024,
        }],
    });
    let mut compute = SgpuComputeSession::new(manager);
    compute.register_resource(SOURCE, source)?;
    compute.register_resource(DESTINATION, destination)?;
    compute.register_kernel(KERNEL, pipeline)?;
    let mut session = SgpuGrantSession::new(compute, CELL, ENDPOINT)?;
    // Independent record authoring, without SgpuCommandList::encode.
    let mut payload = 3u32.to_le_bytes().to_vec();
    payload.push(1);
    for value in [SOURCE, DESTINATION, 1056] {
        payload.extend(value.to_le_bytes());
    }
    for _ in 0..2 {
        payload.push(3);
        for value in [KERNEL, 4, 1, 1, 64, 1, 1] {
            payload.extend(value.to_le_bytes());
        }
    }
    let mut h = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_SUBMIT_COMMAND_LIST);
    h.request_id = 0xfedc_ba98_7654_3210;
    h.object = ENDPOINT;
    h.payload_bytes = payload.len() as u32;
    h.payload_offset = 3;
    h.message_bytes = VF_MSG_BYTES + 3 + h.payload_bytes;
    let inline = SgpuFrame { header: h, payload };
    let inline = SgpuFrame::from_wire(&inline.try_to_wire()?)?;
    let (wire, payload) = inline_to_grant(&inline, GRANT, 16)?;
    let grant = VfGrantEntry {
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
        bytes: 16 + payload.len() as u64 + 8,
        owner_cell: 11,
        state: VF_GRANT_LIVE,
    };
    let mut snapshot = vec![0x99; 9];
    snapshot.extend(payload);
    snapshot.extend([0x55; 8]);
    let views = [VfGrantSnapshot {
        entry: &grant,
        snapshot_offset: 8,
        data: &snapshot[1..],
    }];
    let normalized = session.to_inline(&wire, &views, CELL, 15)?;
    if normalized.payload != payload
        || normalized.header.payload_offset != 0
        || normalized.header.payload_grant != 0
    {
        return Err("authorized inline normalization differs".into());
    }
    let mut runs = Vec::new();
    for run in 0..2u32 {
        let input: Vec<u32> = (0..256u32)
            .map(|i| (i * (123 + run * 14) + 71 + run) ^ 0xff00)
            .collect();
        let mut upload = vec![0x3c; 16];
        upload.extend(input.iter().flat_map(|v| v.to_le_bytes()));
        upload.extend([0xc3; 16]);
        session.compute_mut().write_resource(SOURCE, 0, &upload)?;
        if session.submit(&wire, &views, CELL, 15)? != 3 {
            return Err("wrong completion count".into());
        }
        let readback: Vec<u32> = session
            .compute()
            .read_resource(DESTINATION, 16, 1024)?
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let expected: Vec<u32> = input
            .iter()
            .map(|v| {
                v.wrapping_mul(3)
                    .wrapping_add(7)
                    .wrapping_mul(3)
                    .wrapping_add(7)
            })
            .collect();
        if readback != expected
            || readback == input
            || session.compute().read_resource(DESTINATION, 0, 16)? != [0x3c; 16]
            || session.compute().read_resource(DESTINATION, 1040, 16)? != [0xc3; 16]
        {
            return Err("grant dispatch independent readback or guards differ".into());
        }
        runs.push(serde_json::json!({"input":input,"readback":readback,"all_256_matched":true,"guards_unchanged":true}));
    }
    let before = session.compute().read_resource(DESTINATION, 0, 1056)?;
    let mut rejected = Vec::new();
    for kind in [
        "generation",
        "revoke",
        "owner",
        "read-right",
        "grant-range",
        "expiry",
        "route",
        "snapshot-range",
        "unsupported-opcode",
    ] {
        let mut changed = grant;
        let mut header = decode_grant_header(&wire)?;
        let mut now = 15;
        let mut data = &snapshot[1..];
        match kind {
            "generation" => changed.cap.generation += 1,
            "revoke" => {
                changed.state = 2;
                changed.cap.state = 2;
            }
            "owner" => changed.owner_cell = 0,
            "read-right" => changed.cap.rights = VF_RIGHT_WRITE,
            "grant-range" => changed.bytes = 16 + payload.len() as u64 - 1,
            "expiry" => now = 20,
            "route" => header.object += 1,
            "snapshot-range" => data = &snapshot[1..8],
            _ => header.opcode = VF_SGPU_PRESENT,
        }
        let views = [VfGrantSnapshot {
            entry: &changed,
            snapshot_offset: 8,
            data,
        }];
        let error = session
            .submit(&header.encode(), &views, CELL, now)
            .err()
            .ok_or("invalid grant unexpectedly executed")?;
        if session.compute().read_resource(DESTINATION, 0, 1056)? != before
            || session.compute().completed_command_count() != 6
        {
            return Err("rejected grant changed data or completion count".into());
        }
        rejected.push(serde_json::json!({"kind":kind,"error":error.to_string(),"data_and_completed_count_unchanged":true}));
    }
    if session.compute().manager().vulkan_pipeline_build_count() != Some(1) {
        return Err("same shader was recompiled or pipeline missing".into());
    }
    println!(
        "{}",
        serde_json::json!({"device":device,"runs":runs,"grant_header":wire.as_slice(),"wire_payload":payload,
        "grant_offset":16,"snapshot_offset":8,"unaligned_host_snapshot":true,"root_metadata":"authored trusted-root fixture",
        "completed_commands":6,"gpu_dispatches":4,"software_buffer_copies":2,"native_pipeline_compilations":1,
        "rejected_grants":rejected,"inline_grant_roundtrip_verified":true,"host_vulkan_compute_verified":true,"cpu_fallback":false,
        "actual_vsk_root_submission_verified":false,"actual_guest_driver_submission_verified":false,
        "efi_gpu_command_backend_verified":false,"guest_metal_verified":false,"macos_boot_verified":false})
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("sgpu_grant_vulkan: {error}");
        std::process::exit(1);
    }
}
