/// Opaque handle identifying a GPU-resident texture asset.
///
/// Computed as the FNV-1a hash of the canonical asset path, so the same path
/// always produces the same handle and duplicate requests are deduplicated
/// without a full string comparison at draw time.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetHandle(pub(crate) u64);

impl AssetHandle {
    pub fn from_path(path: &str) -> Self {
        const BASIS: u64 = 14695981039346656037;
        const PRIME: u64 = 1099511628211;
        let mut h = BASIS;
        for byte in path.bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(PRIME);
        }
        Self(h)
    }
}
