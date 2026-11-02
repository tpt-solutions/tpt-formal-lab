//! Tests for `tpt-assurance-case`.

use tpt_assurance_case::{Case, Evidence, EvidenceKind, Status};

fn sorter() -> Case {
    Case::new("Sorter")
        .requirement("REQ-1", "The output is sorted.")
        .requirement("REQ-2", "The output is a permutation of the input.")
        .evidence(
            Evidence::new(
                "T-1",
                EvidenceKind::Test,
                "sorted on random inputs",
                Status::Pass,
            )
            .covers("REQ-1"),
        )
        .evidence(
            Evidence::new(
                "P-1",
                EvidenceKind::Contract,
                "permutation check",
                Status::Pass,
            )
            .covers("REQ-2"),
        )
}

#[test]
fn fully_covered_passing_case_is_supported() {
    let case = sorter();
    assert!(case.is_supported());
    assert!(case.uncovered().is_empty());
    assert!(case.unsuccessful().is_empty());
    assert!(case.dangling().is_empty());
}

#[test]
fn requirement_without_evidence_is_uncovered() {
    let case = Case::new("Gap")
        .requirement("REQ-1", "Covered.")
        .requirement("REQ-2", "Not covered.")
        .evidence(Evidence::new("T-1", EvidenceKind::Test, "t", Status::Pass).covers("REQ-1"));
    assert_eq!(case.uncovered(), vec!["REQ-2"]);
    assert!(!case.is_supported());
}

#[test]
fn failing_evidence_is_reported_and_blocks_support() {
    let case = Case::new("Failing")
        .requirement("REQ-1", "Holds.")
        .evidence(
            Evidence::new(
                "T-1",
                EvidenceKind::Test,
                "t",
                Status::Fail("off by one".into()),
            )
            .covers("REQ-1"),
        );
    let bad = case.unsuccessful();
    assert_eq!(bad.len(), 1);
    assert_eq!(bad[0].id, "T-1");
    assert!(!case.is_supported());
}

#[test]
fn not_run_evidence_is_not_passing() {
    let case = Case::new("Pending")
        .requirement("REQ-1", "Holds.")
        .evidence(Evidence::new("T-1", EvidenceKind::Proof, "p", Status::NotRun).covers("REQ-1"));
    assert!(!case.is_supported());
}

#[test]
fn evidence_naming_an_unknown_requirement_is_dangling() {
    let case = Case::new("Typo")
        .requirement("REQ-1", "Holds.")
        .evidence(Evidence::new("T-1", EvidenceKind::Test, "t", Status::Pass).covers("REQ-9"));
    assert_eq!(case.dangling(), vec![("T-1", "REQ-9")]);
    assert!(!case.is_supported());
}

#[test]
fn one_evidence_can_cover_several_requirements() {
    let case = sorter().evidence(
        Evidence::new("A-1", EvidenceKind::Analysis, "review", Status::Pass)
            .covers("REQ-1")
            .covers("REQ-2"),
    );
    let matrix = case.traceability();
    assert_eq!(matrix[0].evidence.len(), 2);
    assert_eq!(matrix[1].evidence.len(), 2);
}

#[test]
fn traceability_keeps_requirement_order() {
    let case = sorter();
    let ids: Vec<&str> = case
        .traceability()
        .iter()
        .map(|t| t.requirement.id.as_str())
        .collect();
    assert_eq!(ids, ["REQ-1", "REQ-2"]);
}

#[test]
fn markdown_marks_undeveloped_goals_and_failures() {
    let case = Case::new("Mixed")
        .requirement("REQ-1", "Holds.")
        .requirement("REQ-2", "Unproven.")
        .evidence(
            Evidence::new("T-1", EvidenceKind::Test, "t", Status::Fail("boom".into()))
                .covers("REQ-1"),
        );
    let md = case.to_markdown();
    assert!(md.contains("# Assurance case: Mixed"));
    assert!(md.contains("**G0** The system meets all of its requirements. _(not supported)_"));
    assert!(md.contains("**G1** REQ-1: Holds."));
    assert!(md.contains("[FAIL: boom]"));
    assert!(md.contains("**G2** REQ-2: Unproven."));
    assert!(md.contains("_Undeveloped: no evidence._"));
}

#[test]
fn markdown_for_supported_case() {
    let md = sorter().to_markdown();
    assert!(md.contains("_(supported)_"));
    assert!(md.contains("**Sn T-1** [PASS] sorted on random inputs (test)"));
    assert!(!md.contains("Undeveloped"));
}
