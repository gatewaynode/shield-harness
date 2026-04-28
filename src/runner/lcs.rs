// Shared lcs binary resolution. Used by every module that spawns lcs.

use std::path::{Path, PathBuf};

/// Resolve the lcs binary to spawn. If an explicit path is provided, use it
/// verbatim; otherwise fall back to `"lcs"` and rely on PATH lookup at exec time.
pub fn binary(override_path: Option<&Path>) -> PathBuf {
    match override_path {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from("lcs"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_returned_verbatim() {
        let p = Path::new("/opt/lcs/bin/lcs");
        assert_eq!(binary(Some(p)), PathBuf::from("/opt/lcs/bin/lcs"));
    }

    #[test]
    fn no_override_falls_back_to_path_lookup() {
        assert_eq!(binary(None), PathBuf::from("lcs"));
    }
}
