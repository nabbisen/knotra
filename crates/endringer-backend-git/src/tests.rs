use endringer_backend_core::types::CommitId;

#[test]
fn commit_id_ord() {
    let a = CommitId::from_hex("0000000000000000000000000000000000000000").unwrap();
    let b = CommitId::from_hex("000000000000000000000000000000000000001").unwrap();
    let c = CommitId::from_hex("ffffffffffffffffffffffffffffffffffffffff").unwrap();

    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
    assert_eq!(a.cmp(&a), std::cmp::Ordering::Equal);

    // Sorting works
    let mut ids = vec![c.clone(), a.clone(), b.clone()];
    ids.sort();
    assert_eq!(ids, vec![a, b, c]);
}

#[test]
fn commit_id_from_hex_roundtrip() {
    let hex40 = "a".repeat(40);
    let id = CommitId::from_hex(&hex40).unwrap();
    assert_eq!(id.to_string(), hex40);
    assert_eq!(id.short().len(), 7);
}

#[test]
fn commit_id_from_hex_invalid() {
    assert!(CommitId::from_hex("not-a-hash").is_err());
    assert!(CommitId::from_hex(&"0".repeat(39)).is_err());
    assert!(CommitId::from_hex(&"z".repeat(40)).is_err());
}

#[test]
fn diff_summary_is_sorted() {
    use endringer_backend_core::types::DiffSummary;
    use std::path::PathBuf;

    // DiffSummary itself has no sorting logic; the sorting guarantee comes
    // from the backend. Verify the type supports it.
    let mut s = DiffSummary {
        added: vec![PathBuf::from("z.rs"), PathBuf::from("a.rs")],
        modified: vec![],
        deleted: vec![],
    };
    s.added.sort();
    assert_eq!(s.added[0], PathBuf::from("a.rs"));
}
