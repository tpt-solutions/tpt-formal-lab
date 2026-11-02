//! GSN-style assurance cases from requirements and evidence.
//!
//! A [`Case`] lists the requirements a system must meet, and the evidence that
//! supports each one: tests, contract checks, proofs and analyses. Each piece of
//! evidence names the requirements it covers. The case then reports which
//! requirements lack evidence, which evidence failed, and which evidence refers
//! to a requirement that does not exist. [`Case::to_markdown`] renders the
//! argument as a goal structure, and [`Case::traceability`] gives the matrix.
//!
//! Evidence is registered explicitly. This keeps the crate free of external
//! dependencies, and the case is then just data that the build or a test can
//! construct from its own results.
//!
//! # Example
//!
//! ```rust
//! use tpt_assurance_case::{Case, Evidence, EvidenceKind, Status};
//!
//! let case = Case::new("Sorter")
//!     .requirement("REQ-1", "The output is sorted.")
//!     .requirement("REQ-2", "The output is a permutation of the input.")
//!     .evidence(Evidence::new("T-1", EvidenceKind::Test, "sorted on 1000 random inputs", Status::Pass)
//!         .covers("REQ-1"))
//!     .evidence(Evidence::new("P-1", EvidenceKind::Contract, "postcondition checks", Status::Pass)
//!         .covers("REQ-1")
//!         .covers("REQ-2"));
//!
//! assert!(case.is_supported());
//! assert!(case.to_markdown().contains("G2"));
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The kind of evidence, which says how it was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceKind {
    /// A test run, such as a unit, integration or property test.
    Test,
    /// A runtime contract check, such as a `#[requires]` or `#[ensures]` attribute.
    Contract,
    /// A machine-checked proof.
    Proof,
    /// Any other analysis, such as a review or a model check.
    Analysis,
}

impl EvidenceKind {
    fn label(self) -> &'static str {
        match self {
            EvidenceKind::Test => "test",
            EvidenceKind::Contract => "contract",
            EvidenceKind::Proof => "proof",
            EvidenceKind::Analysis => "analysis",
        }
    }
}

/// The outcome of a piece of evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The evidence was produced and passed.
    Pass,
    /// The evidence was produced and failed. The message says why.
    Fail(String),
    /// The evidence has not been run.
    NotRun,
}

impl Status {
    fn label(&self) -> String {
        match self {
            Status::Pass => "PASS".to_owned(),
            Status::Fail(msg) => format!("FAIL: {msg}"),
            Status::NotRun => "NOT RUN".to_owned(),
        }
    }
}

/// A requirement that the case must support.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Requirement {
    /// The requirement identifier, such as `REQ-1`.
    pub id: String,
    /// The statement of the requirement.
    pub statement: String,
}

/// A piece of evidence, with the requirements it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// The evidence identifier, such as `T-1`.
    pub id: String,
    /// How the evidence was produced.
    pub kind: EvidenceKind,
    /// A short description.
    pub description: String,
    /// The outcome.
    pub status: Status,
    /// The requirement identifiers this evidence supports.
    pub covers: Vec<String>,
}

impl Evidence {
    /// Creates evidence that covers no requirement yet. Add coverage with
    /// [`Evidence::covers`].
    pub fn new(
        id: impl Into<String>,
        kind: EvidenceKind,
        description: impl Into<String>,
        status: Status,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            description: description.into(),
            status,
            covers: Vec::new(),
        }
    }

    /// Records that this evidence supports the requirement `requirement_id`.
    #[must_use]
    pub fn covers(mut self, requirement_id: impl Into<String>) -> Self {
        self.covers.push(requirement_id.into());
        self
    }
}

/// One row of the traceability matrix: a requirement and the evidence for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace<'a> {
    /// The requirement.
    pub requirement: &'a Requirement,
    /// The evidence that covers it, in the order it was added.
    pub evidence: Vec<&'a Evidence>,
}

/// An assurance case: requirements, the evidence for them, and the checks on them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Case {
    title: String,
    requirements: Vec<Requirement>,
    evidence: Vec<Evidence>,
}

impl Case {
    /// Creates an empty case with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            requirements: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Adds a requirement.
    #[must_use]
    pub fn requirement(mut self, id: impl Into<String>, statement: impl Into<String>) -> Self {
        self.requirements.push(Requirement {
            id: id.into(),
            statement: statement.into(),
        });
        self
    }

    /// Adds a piece of evidence.
    #[must_use]
    pub fn evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Returns the requirements in the order they were added.
    pub fn requirements(&self) -> &[Requirement] {
        &self.requirements
    }

    /// Returns the evidence in the order it was added.
    pub fn all_evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    /// Returns one trace per requirement, in the order the requirements were added.
    pub fn traceability(&self) -> Vec<Trace<'_>> {
        self.requirements
            .iter()
            .map(|requirement| Trace {
                requirement,
                evidence: self
                    .evidence
                    .iter()
                    .filter(|e| e.covers.contains(&requirement.id))
                    .collect(),
            })
            .collect()
    }

    /// Returns the identifiers of requirements that have no evidence.
    pub fn uncovered(&self) -> Vec<&str> {
        self.traceability()
            .into_iter()
            .filter(|t| t.evidence.is_empty())
            .map(|t| t.requirement.id.as_str())
            .collect()
    }

    /// Returns the evidence whose status is not [`Status::Pass`].
    pub fn unsuccessful(&self) -> Vec<&Evidence> {
        self.evidence
            .iter()
            .filter(|e| e.status != Status::Pass)
            .collect()
    }

    /// Returns `(evidence id, requirement id)` pairs where the evidence names a
    /// requirement that was never added.
    pub fn dangling(&self) -> Vec<(&str, &str)> {
        let known: BTreeSet<&str> = self.requirements.iter().map(|r| r.id.as_str()).collect();
        self.evidence
            .iter()
            .flat_map(|e| {
                e.covers
                    .iter()
                    .filter(|c| !known.contains(c.as_str()))
                    .map(move |c| (e.id.as_str(), c.as_str()))
            })
            .collect()
    }

    /// Returns `true` if every requirement has evidence, every evidence passed,
    /// and no evidence refers to an unknown requirement.
    pub fn is_supported(&self) -> bool {
        self.uncovered().is_empty() && self.unsuccessful().is_empty() && self.dangling().is_empty()
    }

    /// Renders the case as Markdown in a GSN-style goal structure.
    ///
    /// The top goal `G0` claims that every requirement is met. The strategy
    /// `S0` argues over the requirements. Each requirement is a goal `G<n>`, and
    /// each piece of evidence is a solution listed under it with its status.
    /// A requirement with no evidence is marked as undeveloped.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Assurance case: {}\n", self.title);
        let verdict = if self.is_supported() {
            "supported"
        } else {
            "not supported"
        };
        let _ = writeln!(
            out,
            "**G0** The system meets all of its requirements. _({verdict})_"
        );
        let _ = writeln!(out, "\n**S0** Argument over each requirement.\n");
        for (n, trace) in self.traceability().iter().enumerate() {
            let _ = writeln!(
                out,
                "**G{}** {}: {}",
                n + 1,
                trace.requirement.id,
                trace.requirement.statement
            );
            if trace.evidence.is_empty() {
                let _ = writeln!(out, "- _Undeveloped: no evidence._");
            }
            for e in &trace.evidence {
                let _ = writeln!(
                    out,
                    "- **Sn {}** [{}] {} ({})",
                    e.id,
                    e.status.label(),
                    e.description,
                    e.kind.label()
                );
            }
            out.push('\n');
        }
        out
    }
}
