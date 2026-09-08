use std::collections::HashMap;

use thiserror::Error;

use nextcore_gpu::canonical_spec::{GpuCapabilities, GpuVendor, MetalVersion};
use nextcore_gpu::sgpu_command::{SgpuWireError, SGPU_MAX_COMMANDS};
use nextcore_gpu::sgpu_compute::{
    SGPU_MAX_RESOURCES, SGPU_MAX_RESOURCE_BYTES, SGPU_MAX_TOTAL_RESOURCE_BYTES,
};
use nextcore_gpu::virtual_device::{
    CommandBuffer, GpuCommand, VirtualMetalDevice,
};

use crate::vf_abi::{
    VfMsgHeader, VF_FAMILY_SGPU, VF_MSG_BYTES, VF_SGPU_CREATE_RESOURCE,
    VF_SGPU_DESTROY_RESOURCE, VF_SGPU_PRESENT, VF_SGPU_QUERY_CAPS,
    VF_SGPU_SUBMIT_COMMAND_LIST, VF_PAYLOAD_MAX, VfError,
};

/// Golden Gate (macOS 27) graphics service-cell bridge. SGPU family requests
/// from the VSK ABI are translated into the Nextcore GPU command surface and
/// executed on the local software backend. An envelope or an abstraction
/// receipt never constitutes Metal support; only actual backend execution
/// counts, and it reports exactly what it ran.
#[derive(Debug, Error)]
pub enum SgpuError {
    #[error("resource not found: {0}")]
    ResourceNotFound(u64),
    #[error("resource already exists: {0}")]
    ResourceExists(u64),
    #[error("GPU backend error: {0}")]
    Gpu(String),
    #[error("ABI error: {0}")]
    Vf(#[from] VfError),
    #[error("payload decode error")]
    Payload,
    #[error("SGPU resource or transfer allocation limit exceeded")]
    ResourceLimit,
}

impl From<SgpuWireError> for SgpuError {
    fn from(_: SgpuWireError) -> Self { Self::Payload }
}

pub type Result<T> = core::result::Result<T, SgpuError>;

#[derive(Debug, Clone, PartialEq)]
pub struct SgpuCaps {
    pub vendor: GpuVendor,
    pub metal_version: MetalVersion,
    pub vram_bytes: u64,
    pub max_threads_per_threadgroup: u32,
    pub has_compute_shaders: bool,
    pub software_accelerated: bool,
}

impl SgpuCaps {
    pub fn encode(&self) -> [u8; 40] {
        let mut out = [0u8; 40];
        out[0] = self.vendor as u8;
        out[4] = self.metal_version as u8;
        out[8..16].copy_from_slice(&self.vram_bytes.to_le_bytes());
        out[16..20].copy_from_slice(&self.max_threads_per_threadgroup.to_le_bytes());
        out[20] = self.has_compute_shaders as u8;
        out[24] = self.software_accelerated as u8;
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 40 {
            return Err(SgpuError::Payload);
        }
        let vendor = match bytes[0] {
            0 => GpuVendor::AMD,
            1 => GpuVendor::NVIDIA,
            2 => GpuVendor::Intel,
            _ => GpuVendor::Unknown,
        };
        let metal_version = match bytes[4] {
            0 => MetalVersion::V2_0,
            1 => MetalVersion::V3_0,
            2 => MetalVersion::V3_1,
            _ => MetalVersion::V3_2,
        };
        let vram_bytes = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let max_threads = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        Ok(Self {
            vendor,
            metal_version,
            vram_bytes,
            max_threads_per_threadgroup: max_threads,
            has_compute_shaders: bytes[20] != 0,
            software_accelerated: bytes[24] != 0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgpuResource {
    pub sgp_handle: u64,
    pub gpu_handle: u64,
    pub size: u64,
}

pub use nextcore_gpu::sgpu_command::{SgpuCommand, SgpuCommandList};

/// A complete SGPU transfer unit: 64-byte VF header plus bounded payload.
#[derive(Debug, Clone, PartialEq)]
pub struct SgpuFrame {
    pub header: VfMsgHeader,
    pub payload: Vec<u8>,
}

impl SgpuFrame {
    /// Legacy trusted-frame encoder. Use try_to_wire for unchecked input.
    /// Panics when a caller constructs an invalid frame.
    pub fn to_wire(&self) -> Vec<u8> {
        self.try_to_wire().expect("invalid SGPU frame; use try_to_wire for unchecked input")
    }

    /// Validate before any allocation or copy; no truncation or silent omission.
    pub fn try_to_wire(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let (total, range) = frame_layout(&self.header)?;
        let mut out = Vec::new();
        out.try_reserve_exact(total).map_err(|_| SgpuError::ResourceLimit)?;
        out.resize(total, 0);
        out[..VF_MSG_BYTES as usize].copy_from_slice(&self.header.encode());
        out[range].copy_from_slice(&self.payload);
        Ok(out)
    }

    pub fn from_wire(bytes: &[u8]) -> Result<Self> {
        let raw_header = bytes.get(..VF_MSG_BYTES as usize).ok_or(SgpuError::Payload)?;
        let header = VfMsgHeader::decode(raw_header)?;
        let (total, range) = frame_layout(&header)?;
        if bytes.len() != total { return Err(SgpuError::Payload); }
        let source = bytes.get(range).ok_or(SgpuError::Payload)?;
        let mut payload = Vec::new();
        payload.try_reserve_exact(source.len()).map_err(|_| SgpuError::ResourceLimit)?;
        payload.extend_from_slice(source);
        Ok(Self { header, payload })
    }

    /// Applies equally to decoded frames and caller-constructed values.
    pub fn validate(&self) -> Result<()> {
        let (_, range) = frame_layout(&self.header)?;
        if self.payload.len() != range.len() { return Err(SgpuError::Payload); }
        Ok(())
    }
}

fn frame_layout(header: &VfMsgHeader) -> Result<(usize, std::ops::Range<usize>)> {
    header.validate_shape()?;
    if header.family != VF_FAMILY_SGPU {
        return Err(SgpuError::Vf(VfError::Eabi("not an SGPU family frame")));
    }
    // This is APLS's bounded inline body, including its padding. VSK's
    // separately granted payload transport remains a different boundary.
    if u64::from(header.message_bytes) > u64::from(VF_MSG_BYTES) + u64::from(VF_PAYLOAD_MAX) {
        return Err(SgpuError::Vf(VfError::Erange));
    }
    let payload_end = header.payload_end()?;
    let start = u64::from(VF_MSG_BYTES).checked_add(header.payload_offset).ok_or(VfError::Erange)?;
    let end = u64::from(VF_MSG_BYTES).checked_add(payload_end).ok_or(VfError::Erange)?;
    let total = usize::try_from(header.message_bytes).map_err(|_| VfError::Erange)?;
    let start = usize::try_from(start).map_err(|_| VfError::Erange)?;
    let end = usize::try_from(end).map_err(|_| VfError::Erange)?;
    if end > total { return Err(SgpuError::Payload); }
    Ok((total, start..end))
}

/// SGPU service-cell session translating to the Nextcore GPU backend.
#[derive(Debug)]
pub struct SgpuSession {
    pub device: VirtualMetalDevice,
    resources: HashMap<u64, SgpuResource>,
    next_resource_id: u64,
}

impl SgpuSession {
    pub fn new(caps: GpuCapabilities) -> Self {
        Self {
            device: VirtualMetalDevice::new(caps),
            resources: HashMap::new(),
            next_resource_id: 1,
        }
    }

    pub fn query_caps(&self) -> SgpuCaps {
        SgpuCaps {
            vendor: self.device.spec.vendor,
            metal_version: self.device.spec.metal_version,
            vram_bytes: self.device.spec.vram_size,
            max_threads_per_threadgroup: self.device.spec.max_threads,
            // This session has no shader/binding registration or compute backend.
            has_compute_shaders: false,
            software_accelerated: self.device.is_software_accelerated(),
        }
    }

    pub fn create_resource(&mut self, size: u64) -> Result<SgpuResource> {
        if size == 0 {
            return Err(SgpuError::Payload);
        }
        usize::try_from(size).map_err(|_| SgpuError::ResourceLimit)?;
        if size > SGPU_MAX_RESOURCE_BYTES || self.resources.len() >= SGPU_MAX_RESOURCES {
            return Err(SgpuError::ResourceLimit);
        }
        let total = self.resources.values().try_fold(size, |total, resource|
            total.checked_add(resource.size)).ok_or(SgpuError::ResourceLimit)?;
        if total > SGPU_MAX_TOTAL_RESOURCE_BYTES { return Err(SgpuError::ResourceLimit); }
        let next_id = self.next_resource_id.checked_add(1).ok_or(SgpuError::ResourceLimit)?;
        let gpu = self.device.allocate_buffer(size);
        let res = SgpuResource {
            sgp_handle: self.next_resource_id,
            gpu_handle: gpu.handle,
            size: gpu.size,
        };
        self.next_resource_id = next_id;
        self.resources.insert(res.sgp_handle, res.clone());
        Ok(res)
    }

    pub fn destroy_resource(&mut self, handle: u64) -> Result<()> {
        let res = self.resources.remove(&handle).ok_or(SgpuError::ResourceNotFound(handle))?;
        self.device.memory.free(res.gpu_handle).map_err(|e| SgpuError::Gpu(e.to_string()))?;
        Ok(())
    }

    pub fn gpu_handle(&self, sgp_handle: u64) -> Result<u64> {
        self.resources.get(&sgp_handle).map(|r| r.gpu_handle).ok_or(SgpuError::ResourceNotFound(sgp_handle))
    }

    pub fn submit(&mut self, list: &SgpuCommandList) -> Result<usize> {
        if list.cmds.len() > SGPU_MAX_COMMANDS { return Err(SgpuError::Payload); }
        let mut gpu_cmds = Vec::with_capacity(list.cmds.len());
        for cmd in &list.cmds {
            match cmd {
                SgpuCommand::CopyBuffer { src, dst, size } => {
                    let src_gpu = self.gpu_handle(*src)?;
                    let dst_gpu = self.gpu_handle(*dst)?;
                    gpu_cmds.push(GpuCommand::CopyBuffer { src: src_gpu, dst: dst_gpu, size: *size });
                }
                SgpuCommand::RenderClear { color } => {
                    gpu_cmds.push(GpuCommand::RenderClear { color: *color });
                }
                SgpuCommand::ComputeDispatch { kernel_id, grid, block } => {
                    gpu_cmds.push(GpuCommand::ComputeKernel { kernel_id: *kernel_id, grid: *grid, block: *block });
                }
                SgpuCommand::Present { buffer } => {
                    let gpu = self.gpu_handle(*buffer)?;
                    gpu_cmds.push(GpuCommand::PresentSwapchain { buffer: gpu });
                }
            }
        }
        self.device
            .write_command_buffer(CommandBuffer { cmds: gpu_cmds })
            .map_err(|e| SgpuError::Gpu(e.to_string()))?;
        self.device.flush().map_err(|e| SgpuError::Gpu(e.to_string()))?;
        Ok(list.cmds.len())
    }

    pub fn present(&mut self, resource: u64) -> Result<()> {
        self.submit(&SgpuCommandList {
            cmds: vec![SgpuCommand::Present { buffer: resource }],
        })?;
        Ok(())
    }

    /// Deliver an SGPU request frame to the session and produce a response
    /// frame. This is the transport-level bridge: envelope decode → opcode
    /// dispatch → backend execution → response envelope.
    pub fn dispatch(&mut self, frame: &SgpuFrame) -> Result<SgpuFrame> {
        frame.validate()?;

        let resp = |opcode: u16, request_id: u64, payload: Vec<u8>| -> SgpuFrame {
            let mut header = VfMsgHeader::new(VF_FAMILY_SGPU, opcode);
            header.request_id = request_id;
            header.payload_offset = 0;
            header.payload_bytes = payload.len() as u32;
            header.message_bytes = VF_MSG_BYTES + header.payload_bytes;
            SgpuFrame { header, payload }
        };

        match frame.header.opcode {
            VF_SGPU_QUERY_CAPS => {
                if !frame.payload.is_empty() { return Err(SgpuError::Payload); }
                let caps = self.query_caps();
                Ok(resp(VF_SGPU_QUERY_CAPS, frame.header.request_id, caps.encode().to_vec()))
            }
            VF_SGPU_CREATE_RESOURCE => {
                if frame.payload.len() != 8 {
                    return Err(SgpuError::Payload);
                }
                let size = u64::from_le_bytes(frame.payload[0..8].try_into().unwrap());
                let res = self.create_resource(size)?;
                Ok(resp(VF_SGPU_CREATE_RESOURCE, frame.header.request_id, res.sgp_handle.to_le_bytes().to_vec()))
            }
            VF_SGPU_DESTROY_RESOURCE => {
                if frame.payload.len() != 8 {
                    return Err(SgpuError::Payload);
                }
                let handle = u64::from_le_bytes(frame.payload[0..8].try_into().unwrap());
                self.destroy_resource(handle)?;
                Ok(resp(VF_SGPU_DESTROY_RESOURCE, frame.header.request_id, Vec::new()))
            }
            VF_SGPU_SUBMIT_COMMAND_LIST => {
                let list = SgpuCommandList::decode(&frame.payload).map_err(|_| SgpuError::Payload)?;
                let completed = self.submit(&list)?;
                Ok(resp(VF_SGPU_SUBMIT_COMMAND_LIST, frame.header.request_id, (completed as u32).to_le_bytes().to_vec()))
            }
            VF_SGPU_PRESENT => {
                if frame.payload.len() != 8 {
                    return Err(SgpuError::Payload);
                }
                let res = u64::from_le_bytes(frame.payload[0..8].try_into().unwrap());
                self.present(res)?;
                Ok(resp(VF_SGPU_PRESENT, frame.header.request_id, Vec::new()))
            }
            _ => Err(SgpuError::Vf(VfError::Eabi("unknown SGPU opcode"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextcore_gpu::canonical_spec::GpuCapabilities;

    fn software_caps() -> GpuCapabilities {
        GpuCapabilities::software_fallback(GpuVendor::AMD, 4 << 20)
    }

    fn request(opcode: u16, payload: Vec<u8>) -> SgpuFrame {
        let mut header = VfMsgHeader::new(VF_FAMILY_SGPU, opcode);
        header.payload_bytes = payload.len() as u32;
        header.message_bytes = VF_MSG_BYTES + header.payload_bytes;
        SgpuFrame { header, payload }
    }

    #[test]
    fn shared_codec_errors_propagate_into_legacy_result_alias() {
        fn parse_question(bytes: &[u8]) -> Result<SgpuCommandList> {
            Ok(SgpuCommandList::decode(bytes)?)
        }
        fn parse_direct(bytes: &[u8]) -> Result<SgpuCommandList> {
            SgpuCommandList::decode(bytes).map_err(Into::into)
        }
        assert!(parse_question(&[0; 4]).unwrap().cmds.is_empty());
        assert!(parse_direct(&[0; 4]).unwrap().cmds.is_empty());
        assert!(matches!(parse_question(&u32::MAX.to_le_bytes()), Err(SgpuError::Payload)));
        assert!(matches!(parse_direct(&[0; 3]), Err(SgpuError::Payload)));
    }

    #[test]
    fn inline_frame_roundtrip_preserves_header_and_payload_with_padding() {
        let mut frame = request(VF_SGPU_CREATE_RESOURCE, 256u64.to_le_bytes().to_vec());
        frame.header.payload_offset = 3;
        frame.header.message_bytes += 8;
        frame.header.request_id = 0xfedc_ba98_7654_3210;
        frame.header.object = 0x1234_5678_0000_0001;
        frame.header.payload_grant = 0x8877_6655_4433_2211;
        frame.header.flags = 0x1020_3040;
        frame.header.reserved0 = 0x0102_0304_0506_0708;
        let wire = frame.try_to_wire().unwrap();
        assert_eq!(wire.len(), 80);
        assert_eq!(&wire[64..67], &[0; 3]);
        assert_eq!(&wire[75..], &[0; 5]);
        let decoded = SgpuFrame::from_wire(&wire).unwrap();
        assert_eq!(decoded.header.encode(), frame.header.encode());
        assert_eq!(decoded.payload, frame.payload);
        assert_eq!(decoded.to_wire(), wire);
    }

    #[test]
    fn inline_frame_rejects_every_truncation_and_extra_transfer_bytes() {
        let frame = request(VF_SGPU_CREATE_RESOURCE, 8u64.to_le_bytes().to_vec());
        let wire = frame.try_to_wire().unwrap();
        for length in 0..wire.len() {
            assert!(SgpuFrame::from_wire(&wire[..length]).is_err(), "accepted length {length}");
        }
        let mut trailing = wire.clone();
        trailing.push(0);
        assert!(SgpuFrame::from_wire(&trailing).is_err());
        for declared in [64u32, 71, 73, u32::MAX] {
            let mut bad = wire.clone();
            bad[8..12].copy_from_slice(&declared.to_le_bytes());
            assert!(SgpuFrame::from_wire(&bad).is_err());
        }
        // Previously this query was accepted with its missing payload erased.
        let mut query = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_QUERY_CAPS);
        query.message_bytes = 72;
        query.payload_bytes = 8;
        assert!(SgpuFrame::from_wire(&query.encode()).is_err());
    }

    #[test]
    fn inline_frame_checks_overflow_limits_and_typed_payload_consistency() {
        let valid = request(VF_SGPU_CREATE_RESOURCE, 8u64.to_le_bytes().to_vec());
        for offset in [u64::MAX, u64::MAX - 3, 1u64 << 32] {
            let mut bad = valid.clone();
            bad.header.payload_offset = offset;
            assert!(bad.try_to_wire().is_err());
            assert!(SgpuFrame::from_wire(&bad.header.encode()).is_err());
        }
        let mut bad = valid.clone();
        bad.header.message_bytes = VF_MSG_BYTES + VF_PAYLOAD_MAX + 1;
        assert!(bad.try_to_wire().is_err());
        bad = valid.clone();
        bad.header.payload_bytes = VF_PAYLOAD_MAX + 1;
        assert!(bad.try_to_wire().is_err());
        bad = valid.clone();
        bad.payload.push(0);
        assert!(bad.try_to_wire().is_err());
        bad = valid.clone();
        bad.header.magic = 0;
        assert!(bad.try_to_wire().is_err());
        bad = valid;
        bad.header.family = crate::vf_abi::VF_FAMILY_ROOT;
        assert!(bad.try_to_wire().is_err());
    }

    #[test]
    fn dispatch_validates_direct_frames_before_resource_mutation() {
        let mut session = SgpuSession::new(software_caps());
        let mut bad = request(VF_SGPU_CREATE_RESOURCE, 8u64.to_le_bytes().to_vec());
        bad.header.payload_bytes = 0;
        assert!(session.dispatch(&bad).is_err());
        bad.header.payload_bytes = 8;
        bad.header.message_bytes = 64;
        assert!(session.dispatch(&bad).is_err());
        let resource = session.create_resource(8).unwrap();
        assert_eq!(resource.sgp_handle, 1);
        assert_eq!(resource.gpu_handle, 1);
    }

    #[test]
    fn request_opcodes_require_exact_payload_lengths() {
        let mut session = SgpuSession::new(software_caps());
        let resource = session.create_resource(8).unwrap();
        for opcode in [VF_SGPU_CREATE_RESOURCE, VF_SGPU_DESTROY_RESOURCE, VF_SGPU_PRESENT] {
            for length in [7, 9] {
                let mut payload = resource.sgp_handle.to_le_bytes().to_vec();
                payload.resize(length, 0);
                assert!(matches!(session.dispatch(&request(opcode, payload)), Err(SgpuError::Payload)));
            }
        }
        assert!(matches!(session.dispatch(&request(VF_SGPU_QUERY_CAPS, vec![0])), Err(SgpuError::Payload)));
        assert_eq!(session.gpu_handle(resource.sgp_handle).unwrap(), resource.gpu_handle);
    }

    #[test]
    fn oversized_resource_requests_fail_without_entering_allocator() {
        let mut session = SgpuSession::new(software_caps());
        for size in [SGPU_MAX_RESOURCE_BYTES + 1, 1u64 << 32, u64::MAX] {
            let wire = request(VF_SGPU_CREATE_RESOURCE, size.to_le_bytes().to_vec()).try_to_wire().unwrap();
            let decoded = SgpuFrame::from_wire(&wire).unwrap();
            assert!(matches!(session.dispatch(&decoded), Err(SgpuError::ResourceLimit)));
        }
        assert!(matches!(session.create_resource(0), Err(SgpuError::Payload)));
        let resource = session.create_resource(4).unwrap();
        assert_eq!(resource.sgp_handle, 1);
        assert_eq!(resource.gpu_handle, 1);
    }

    #[test]
    fn resource_count_budget_recovers_after_destroy() {
        let mut session = SgpuSession::new(software_caps());
        let resources: Vec<_> = (0..SGPU_MAX_RESOURCES).map(|_| session.create_resource(4).unwrap()).collect();
        assert!(matches!(session.create_resource(4), Err(SgpuError::ResourceLimit)));
        session.destroy_resource(resources[0].sgp_handle).unwrap();
        assert!(session.gpu_handle(resources[0].sgp_handle).is_err());
        assert!(session.device.memory.read(resources[0].gpu_handle, 0, 1).is_err());
        let replacement = session.create_resource(4).unwrap();
        assert_eq!(replacement.sgp_handle, SGPU_MAX_RESOURCES as u64 + 1);
    }

    #[test]
    fn resource_byte_budget_recovers_after_destroy() {
        let mut session = SgpuSession::new(software_caps());
        let count = SGPU_MAX_TOTAL_RESOURCE_BYTES / SGPU_MAX_RESOURCE_BYTES;
        let resources: Vec<_> = (0..count).map(|_| session.create_resource(SGPU_MAX_RESOURCE_BYTES).unwrap()).collect();
        assert!(matches!(session.create_resource(1), Err(SgpuError::ResourceLimit)));
        session.destroy_resource(resources[0].sgp_handle).unwrap();
        session.create_resource(SGPU_MAX_RESOURCE_BYTES).unwrap();
        assert!(matches!(session.create_resource(1), Err(SgpuError::ResourceLimit)));
    }

    #[test]
    fn shared_codec_rejects_guest_count_attack_and_trailing_record_data() {
        assert!(SgpuCommandList::decode(&u32::MAX.to_le_bytes()).is_err());
        let mut payload = SgpuCommand::Present { buffer: 1 }.encode();
        payload.push(0);
        assert!(SgpuCommand::decode(&payload).is_err());
    }

    #[test]
    fn caps_roundtrip() {
        let caps = SgpuCaps {
            vendor: GpuVendor::AMD,
            metal_version: MetalVersion::V3_1,
            vram_bytes: 4 << 20,
            max_threads_per_threadgroup: 1,
            has_compute_shaders: false,
            software_accelerated: true,
        };
        assert_eq!(SgpuCaps::decode(&caps.encode()).unwrap(), caps);
    }

    #[test]
    fn command_roundtrip_all_variants() {
        let cmds = SgpuCommandList {
            cmds: vec![
                SgpuCommand::CopyBuffer { src: 1, dst: 2, size: 256 },
                SgpuCommand::RenderClear { color: [1.0, 0.5, 0.25, 0.0] },
                SgpuCommand::ComputeDispatch { kernel_id: 7, grid: (64, 1, 1), block: (8, 8, 1) },
                SgpuCommand::Present { buffer: 3 },
            ],
        };
        let decoded = SgpuCommandList::decode(&cmds.encode()).unwrap();
        assert_eq!(decoded, cmds);
        let trunc = cmds.encode();
        assert!(SgpuCommandList::decode(&trunc[..trunc.len() - 1]).is_err());
    }

    #[test]
    fn copy_buffer_executes_on_software_backend() {
        let mut session = SgpuSession::new(software_caps());
        let src = session.create_resource(256).unwrap();
        let dst = session.create_resource(256).unwrap();
        // Write pattern into source GPU buffer directly.
        session.device.memory.write(src.gpu_handle, 0, &[0xEEu8; 256]).unwrap();
        session
            .submit(&SgpuCommandList {
                cmds: vec![SgpuCommand::CopyBuffer { src: src.sgp_handle, dst: dst.sgp_handle, size: 256 }],
            })
            .unwrap();
        let data = session.device.memory.read(dst.gpu_handle, 0, 256).unwrap();
        assert!(data.iter().all(|&b| b == 0xEE));
    }

    #[test]
    fn dispatch_query_caps_through_vf_frame() {
        let mut session = SgpuSession::new(software_caps());
        let mut h = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_QUERY_CAPS);
        h.request_id = 42;
        h.payload_offset = 0;
        h.message_bytes = VF_MSG_BYTES;
        h.payload_bytes = 0;
        let req = SgpuFrame { header: h, payload: Vec::new() };
        let resp = session.dispatch(&req).unwrap();
        assert_eq!(resp.header.request_id, 42);
        assert_eq!(resp.header.opcode, VF_SGPU_QUERY_CAPS);
        assert!(resp.header.message_bytes >= VF_MSG_BYTES);
        let caps = SgpuCaps::decode(&resp.payload).unwrap();
        assert!(caps.software_accelerated);
        assert_eq!(caps.vendor, GpuVendor::AMD);
    }

    #[test]
    fn dispatch_create_and_copy_via_wire_roundtrip() {
        let mut session = SgpuSession::new(software_caps());

        let mut h = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_CREATE_RESOURCE);
        h.request_id = 1;
        h.payload_offset = 0;
        h.payload_bytes = 8;
        h.message_bytes = VF_MSG_BYTES + 8;
        let req = SgpuFrame { header: h, payload: 256u64.to_le_bytes().to_vec() };
        let wire = req.to_wire();
        let back = SgpuFrame::from_wire(&wire).unwrap();
        let resp = session.dispatch(&back).unwrap();
        let src = u64::from_le_bytes(resp.payload[0..8].try_into().unwrap());

        let mut h = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_CREATE_RESOURCE);
        h.request_id = 2;
        h.payload_offset = 0;
        h.payload_bytes = 8;
        h.message_bytes = VF_MSG_BYTES + 8;
        let req = SgpuFrame { header: h, payload: 256u64.to_le_bytes().to_vec() };
        let resp = session.dispatch(&req).unwrap();
        let dst = u64::from_le_bytes(resp.payload[0..8].try_into().unwrap());

        session.device.memory.write(session.gpu_handle(src).unwrap(), 0, &[0xABu8; 256]).unwrap();

        let list = SgpuCommandList { cmds: vec![SgpuCommand::CopyBuffer { src, dst, size: 256 }] };
        let payload = list.encode();
        let mut h = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_SUBMIT_COMMAND_LIST);
        h.request_id = 3;
        h.payload_offset = 0;
        h.payload_bytes = payload.len() as u32;
        h.message_bytes = VF_MSG_BYTES + h.payload_bytes;
        let req = SgpuFrame { header: h, payload };
        let resp = session.dispatch(&req).unwrap();
        assert_eq!(u32::from_le_bytes(resp.payload[0..4].try_into().unwrap()), 1);

        let data = session.device.memory.read(session.gpu_handle(dst).unwrap(), 0, 256).unwrap();
        assert!(data.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn present_without_backend_is_rejected() {
        let mut session = SgpuSession::new(software_caps());
        let res = session.create_resource(64).unwrap();
        assert!(matches!(session.present(res.sgp_handle), Err(SgpuError::Gpu(_))));
    }

    #[test]
    fn unsupported_dispatch_has_no_success_frame_or_partial_copy() {
        let mut session = SgpuSession::new(GpuCapabilities::default_amd());
        assert!(!session.query_caps().has_compute_shaders);
        assert!(!session.device.supports_metal());
        let src = session.create_resource(4).unwrap();
        let dst = session.create_resource(4).unwrap();
        session.device.memory.write(src.gpu_handle, 0, &[1, 2, 3, 4]).unwrap();
        for unsupported in [
            SgpuCommand::ComputeDispatch { kernel_id: 7, grid: (1, 1, 1), block: (1, 1, 1) },
            SgpuCommand::RenderClear { color: [1.0; 4] },
            SgpuCommand::Present { buffer: dst.sgp_handle },
        ] {
            let payload = SgpuCommandList { cmds: vec![
                SgpuCommand::CopyBuffer { src: src.sgp_handle, dst: dst.sgp_handle, size: 4 },
                unsupported,
            ] }.encode();
            let mut header = VfMsgHeader::new(VF_FAMILY_SGPU, VF_SGPU_SUBMIT_COMMAND_LIST);
            header.payload_bytes = payload.len() as u32;
            header.message_bytes = VF_MSG_BYTES + header.payload_bytes;
            let decoded = SgpuFrame::from_wire(&SgpuFrame { header, payload }.to_wire()).unwrap();
            assert!(matches!(session.dispatch(&decoded), Err(SgpuError::Gpu(_))));
            assert_eq!(session.device.memory.read(dst.gpu_handle, 0, 4).unwrap(), [0; 4]);
        }
    }

    #[test]
    fn unknown_opcode_rejected() {
        let mut session = SgpuSession::new(software_caps());
        let h = VfMsgHeader::new(VF_FAMILY_SGPU, 0xF0);
        let frame = SgpuFrame { header: h, payload: Vec::new() };
        assert!(session.dispatch(&frame).is_err());
    }

    #[test]
    fn unknown_resource_rejected() {
        let mut session = SgpuSession::new(software_caps());
        let list = SgpuCommandList { cmds: vec![SgpuCommand::CopyBuffer { src: 99, dst: 100, size: 1 }] };
        assert!(matches!(session.submit(&list), Err(SgpuError::ResourceNotFound(99))));
    }
}
