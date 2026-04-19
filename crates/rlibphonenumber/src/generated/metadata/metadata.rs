// This file is auto-generated. Do not edit.
#[repr(C, align(16))]
struct AlignedBytes<const N: usize>(pub [u8; N]);
static METADATA_BYTES: AlignedBytes<{ include_bytes!("metadata.bin").len() }> = AlignedBytes(*include_bytes!("metadata.bin"));
pub const METADATA: &[u8] = &METADATA_BYTES.0;
