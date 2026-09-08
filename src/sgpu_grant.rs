//! Existing VSK grant transport to the shared SGPU codec/compute executor.
//! See SGPU_GRANT_TRANSPORT.md: trusted root metadata and canonical references
//! must remain current/stable for this synchronous call. No raw pointer mapping.
use crate::{sgpu::SgpuFrame, vf_abi::*};
use nextcore_gpu::sgpu_compute::{SgpuComputeSession, SgpuSubmissionError};
use std::result::Result;
use thiserror::Error;

pub const VF_POLICY_TABLE_MAX: usize = 4096;
pub const VF_RIGHT_READ: u32 = 1;
pub const VF_RIGHT_WRITE: u32 = 2;
pub const VF_RIGHT_ALL: u32 = 255;
pub const VF_OBJECT_GRANT: u32 = 1;
pub const VF_CAP_LIVE: u32 = 1;
pub const VF_GRANT_LIVE: u32 = 1;

/// Exact signed statuses used by the VSK C checks performed here.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum GrantError {
    #[error("invalid grant argument")]
    Einval = -1,
    #[error("VSK grant header ABI mismatch")]
    Eabi = -2,
    #[error("unsupported operation")]
    Enotsup = -3,
    #[error("stale grant capability or lifecycle")]
    Estale = -4,
    #[error("grant permission or route denied")]
    Eperm = -5,
    #[error("grant or snapshot range exceeded")]
    Erange = -6,
    #[error("ambiguous caller-scoped grant slot")]
    Eambig = -9,
}

/// Root metadata, not another wire structure. Fields match vf_cap_entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VfCapEntry {
    pub slot: u32,
    pub generation: u32,
    pub holder_cell: u32,
    pub object_type: u32,
    pub rights: u32,
    pub state: u32,
    pub object_id: u64,
    pub valid_from: u64,
    pub expires_at: u64,
}

/// The policy-relevant projection of a locked canonical vf_grant_entry.
/// Refcounts, DMA/page flags and revocation acknowledgements remain root-owned;
/// vf_validate_payload_grant itself does not inspect them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VfGrantEntry {
    pub cap: VfCapEntry,
    pub bytes: u64,
    pub owner_cell: u32,
    pub state: u32,
}

/// Caller-owned immutable window, bound by trusted root to this canonical grant.
/// `snapshot_offset` is the window's byte position in that grant, not a pointer.
#[derive(Debug, Clone, Copy)]
pub struct VfGrantSnapshot<'a> {
    pub entry: &'a VfGrantEntry,
    pub snapshot_offset: u64,
    pub data: &'a [u8],
}

/// Exact VSK shape semantics. APLS's older validate_shape/payload_end methods
/// describe the inline profile and must not be used for granted messages.
pub fn validate_grant_header(h: &VfMsgHeader) -> Result<(), GrantError> {
    if h.magic != VF_MSG_MAGIC
        || h.abi_major != VF_ABI_MAJOR
        || h.header_bytes != VF_MSG_BYTES as u16
        || h.message_bytes != VF_MSG_BYTES
        || h.reserved0 != 0
    {
        return Err(GrantError::Eabi);
    }
    if h.flags != 0 {
        return Err(GrantError::Einval);
    }
    let known = match h.family {
        VF_FAMILY_ROOT => (1..=9).contains(&h.opcode),
        VF_FAMILY_BLOCK => (1..=3).contains(&h.opcode),
        VF_FAMILY_SGPU => (1..=7).contains(&h.opcode),
        _ => false,
    };
    if !known {
        return Err(GrantError::Enotsup);
    }
    if h.payload_bytes == 0 {
        return if h.payload_grant == 0 && h.payload_offset == 0 {
            Ok(())
        } else {
            Err(GrantError::Einval)
        };
    }
    if h.payload_grant == 0 || h.payload_offset & 7 != 0 {
        return Err(GrantError::Einval);
    }
    if h.payload_bytes > VF_PAYLOAD_MAX
        || h.payload_offset
            .checked_add(u64::from(h.payload_bytes))
            .is_none()
    {
        return Err(GrantError::Erange);
    }
    Ok(())
}

pub fn decode_grant_header(bytes: &[u8]) -> Result<VfMsgHeader, GrantError> {
    let header = VfMsgHeader::decode(bytes).map_err(|_| GrantError::Eabi)?;
    validate_grant_header(&header)?;
    Ok(header)
}

/// Mirrors vf_validate_payload_grant, including duplicate-slot and error order.
/// This checks policy only; snapshot coverage is checked separately before use.
pub fn validate_payload_grant<'a, 'b>(
    header: &VfMsgHeader,
    table: &'a [VfGrantSnapshot<'b>],
    caller_cell: u32,
    required_rights: u32,
    now: u64,
) -> Result<Option<&'a VfGrantSnapshot<'b>>, GrantError> {
    if table.len() > VF_POLICY_TABLE_MAX
        || caller_cell == 0
        || required_rights == 0
        || required_rights & !(VF_RIGHT_READ | VF_RIGHT_WRITE) != 0
    {
        return Err(GrantError::Einval);
    }
    validate_grant_header(header)?;
    if header.payload_bytes == 0 {
        return Ok(None);
    }
    let mut found = None;
    for snapshot in table {
        let cap = &snapshot.entry.cap;
        if cap.holder_cell == caller_cell && cap.slot == header.payload_grant as u32 {
            if found.is_some() {
                return Err(GrantError::Eambig);
            }
            found = Some(snapshot);
        }
    }
    let snapshot = found.ok_or(GrantError::Estale)?;
    let grant = snapshot.entry;
    let cap = &grant.cap;
    let handle = (u64::from(cap.generation) << 32) | u64::from(cap.slot);
    if cap.slot == 0
        || cap.generation == 0
        || handle != header.payload_grant
        || cap.state != VF_CAP_LIVE
        || cap.valid_from >= cap.expires_at
        || now < cap.valid_from
        || now >= cap.expires_at
    {
        return Err(GrantError::Estale);
    }
    if cap.object_id == 0
        || cap.object_type != VF_OBJECT_GRANT
        || cap.rights & !VF_RIGHT_ALL != 0
        || required_rights & !cap.rights != 0
    {
        return Err(GrantError::Eperm);
    }
    if grant.state != VF_GRANT_LIVE || grant.owner_cell == 0 {
        return Err(GrantError::Estale);
    }
    if header.payload_offset > grant.bytes
        || u64::from(header.payload_bytes) > grant.bytes - header.payload_offset
    {
        return Err(GrantError::Erange);
    }
    Ok(Some(snapshot))
}

fn payload_window<'a>(
    header: &VfMsgHeader,
    snapshot: &VfGrantSnapshot<'a>,
) -> Result<&'a [u8], GrantError> {
    let window_len = u64::try_from(snapshot.data.len()).map_err(|_| GrantError::Erange)?;
    let window_end = snapshot
        .snapshot_offset
        .checked_add(window_len)
        .ok_or(GrantError::Erange)?;
    if window_end > snapshot.entry.bytes {
        return Err(GrantError::Erange);
    }
    let start = header
        .payload_offset
        .checked_sub(snapshot.snapshot_offset)
        .ok_or(GrantError::Erange)?;
    let end = start
        .checked_add(u64::from(header.payload_bytes))
        .ok_or(GrantError::Erange)?;
    let range = usize::try_from(start).map_err(|_| GrantError::Erange)?
        ..usize::try_from(end).map_err(|_| GrantError::Erange)?;
    snapshot.data.get(range).ok_or(GrantError::Erange)
}

#[derive(Debug, Error)]
pub enum GrantSubmitError {
    #[error(transparent)]
    Grant(#[from] GrantError),
    #[error("inline SGPU frame: {0}")]
    Inline(#[from] crate::sgpu::SgpuError),
    #[error(transparent)]
    Submission(#[from] SgpuSubmissionError),
}

/// Pure conversion: the root still has to publish/copy the returned payload into
/// its grant. This function creates no grant or authority and touches no memory
/// outside the existing inline frame.
pub fn inline_to_grant(
    frame: &SgpuFrame,
    grant_handle: u64,
    grant_offset: u64,
) -> Result<([u8; 64], &[u8]), GrantSubmitError> {
    frame.validate()?;
    let mut header = frame.header;
    header.message_bytes = VF_MSG_BYTES;
    header.payload_grant = grant_handle;
    header.payload_offset = grant_offset;
    validate_grant_header(&header)?;
    Ok((header.encode(), &frame.payload))
}

/// Trusted-root route binding; construction does not authenticate an endpoint.
/// The caller must authorize the endpoint independently using its actual router
/// policy, then retain that authorization and the canonical grant reference for
/// each synchronous operation. No session accepts authority from inline frames.
pub struct SgpuGrantSession {
    caller_cell: u32,
    endpoint_handle: u64,
    compute: SgpuComputeSession,
}

impl SgpuGrantSession {
    pub fn new(
        compute: SgpuComputeSession,
        caller_cell: u32,
        endpoint_handle: u64,
    ) -> Result<Self, GrantError> {
        if caller_cell == 0 || endpoint_handle as u32 == 0 || endpoint_handle >> 32 == 0 {
            return Err(GrantError::Einval);
        }
        Ok(Self {
            caller_cell,
            endpoint_handle,
            compute,
        })
    }

    fn authorize<'a>(
        &self,
        header_bytes: &[u8],
        table: &[VfGrantSnapshot<'a>],
        caller_cell: u32,
        now: u64,
    ) -> Result<(VfMsgHeader, &'a [u8]), GrantError> {
        let header = decode_grant_header(header_bytes)?;
        if header.family != VF_FAMILY_SGPU || header.opcode != VF_SGPU_SUBMIT_COMMAND_LIST {
            return Err(GrantError::Enotsup);
        }
        if caller_cell != self.caller_cell || header.object != self.endpoint_handle {
            return Err(GrantError::Eperm);
        }
        let snapshot = validate_payload_grant(&header, table, caller_cell, VF_RIGHT_READ, now)?;
        let payload = match snapshot {
            Some(snapshot) => payload_window(&header, snapshot)?,
            None => &[], // Shared codec rejects a missing command-count field.
        };
        Ok((header, payload))
    }

    pub fn submit(
        &mut self,
        header: &[u8],
        table: &[VfGrantSnapshot<'_>],
        caller_cell: u32,
        now: u64,
    ) -> Result<usize, GrantSubmitError> {
        let (_, payload) = self.authorize(header, table, caller_cell, now)?;
        Ok(self.compute.submit_wire(payload, 0, payload.len() as u64)?)
    }

    /// Authorized data normalization, not an authority token or submission.
    pub fn to_inline(
        &self,
        header: &[u8],
        table: &[VfGrantSnapshot<'_>],
        caller_cell: u32,
        now: u64,
    ) -> Result<SgpuFrame, GrantSubmitError> {
        let (mut header, payload) = self.authorize(header, table, caller_cell, now)?;
        header.message_bytes = VF_MSG_BYTES
            .checked_add(header.payload_bytes)
            .ok_or(GrantError::Erange)?;
        header.payload_grant = 0;
        header.payload_offset = 0;
        let mut copy = Vec::new();
        copy.try_reserve_exact(payload.len())
            .map_err(|_| crate::sgpu::SgpuError::ResourceLimit)?;
        copy.extend_from_slice(payload);
        let frame = SgpuFrame {
            header,
            payload: copy,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn compute(&self) -> &SgpuComputeSession {
        &self.compute
    }
    pub fn compute_mut(&mut self) -> &mut SgpuComputeSession {
        &mut self.compute
    }
    pub fn into_compute(self) -> SgpuComputeSession {
        self.compute
    }
}
