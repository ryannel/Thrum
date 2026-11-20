//! Compare authority and real runtime publications with prediction enabled and disabled.

#[test]
fn prediction_preserves_all_peer_digests_and_publication_pages() {
    let run = match_proof::run_prediction("isolated", "a0", 240, 128, 29_721)
        .expect("valid conditioned prediction fixture");
    assert_eq!(
        run["authority"]
            .as_array()
            .expect("valid conditioned prediction fixture")
            .len(),
        240
    );
    assert_eq!(
        run["replicas"]
            .as_array()
            .expect("valid conditioned prediction fixture")
            .len(),
        4
    );
}
