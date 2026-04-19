// This file is auto-generated. Do not edit.
#[repr(C, align(16))]
struct AlignedBytes<const N: usize>(pub [u8; N]);
static METADATA_BYTES: AlignedBytes<{ include_bytes!("test_metadata.bin").len() }> = AlignedBytes(*include_bytes!("test_metadata.bin"));
pub const TEST_METADATA: &[u8] = &METADATA_BYTES.0;
