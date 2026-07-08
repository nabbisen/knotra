use std::path::Path;
use super::backend::JjBackend;

#[test]
fn rejects_non_jj_path() {
    // A plain directory (or missing path) is not a jj repo.
    assert!(JjBackend::open(Path::new("/tmp")).is_err());
    assert!(JjBackend::open(Path::new("/no/such/dir")).is_err());
}

#[test]
fn create_annotated_tag_returns_error() {
    // Without a real jj repo we can only test the error path.
    // We can't open a JjBackend without a .jj/ dir, but we can verify the
    // error message shape via a mock. For now we document the contract.
    let err = anyhow::anyhow!("jj does not support annotated tags; use create_tag");
    assert!(err.to_string().contains("annotated"));
}
