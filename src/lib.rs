pub mod authority;
pub mod context_assembly;
pub mod contracts;
pub mod durable_evidence;
pub mod enums;
pub mod events;
pub mod evidence_bundle;
pub mod evidence_store;
pub mod fixture_bundle;
#[cfg(any(test, feature = "test-support"))]
pub mod fixture_support;
pub mod invalidation_engine;
pub mod models;
pub mod proof;
pub mod proof_bundle;
pub mod remediation_flow;
pub mod replay;
pub mod schema_bundle;
pub mod schema_validation;
pub mod state_machine;

pub use authority::AuthorityResolutionRecord;
pub use context_assembly::{
    assemble_context, AuthorityState, ClassFreshnessOverride, ContextAssemblyError,
    ContextAssemblyOutput, ContextAssemblyRequest, ContextBundleManifest, FreshnessBand,
    FreshnessPolicy, OverrideDecision, OverridePosture, ReplayEligibility, SourceClass,
    SourceInput, SourceInventoryEntry, SourceProvenance, TargetRefs,
};
pub use contracts::{
    KeyFilePacketContract, RepoNavigationAssistPacketContract, RepoNavigationMapContract,
    ValidationCommandPacketContract,
};
pub use durable_evidence::{
    ArtifactInvalidationEvidenceRecord, CoalescedBatchRecord, EventReceiptRecord,
    EvidenceAdmissionResult, EvidenceBatchOutcome, EvidenceTargetKind,
    PacketReevaluationEvidenceRecord, RemediationEvidenceRecord, ReplayBundleManifest,
};
pub use enums::*;
pub use events::{EventBatch, EventLedger, EventProcessingDecision, EventRecord};
pub use evidence_bundle::{
    build_and_write_replay_bundle, build_replay_bundle_manifest, load_evidence_bundle,
    EvidenceBundleData, EvidenceBundleError,
};
pub use evidence_store::{EvidenceStore, EvidenceStoreError};
pub use invalidation_engine::{
    apply_artifact_invalidation, apply_packet_constituent_change, ArtifactInvalidationDecision,
    ArtifactInvalidationOutcome, PacketInvalidationOutcome,
};
pub use models::{ArtifactRecord, OverrideRecord, PacketRecord, RemediationItem};
pub use proof::report::{GovernedFlowReport, GovernedFlowStep};
pub use proof::{
    export_proof_package, run_governed_flow_proof, run_replay_scenario_proof, ProofExportError,
    ProofExportReport, ReplayScenarioError, ReplayScenarioReport,
};
pub use remediation_flow::{
    plan_artifact_remediation, plan_packet_remediation, remediation_required_for_packet,
    RemediationPlan, RemediationTrigger,
};
pub use replay::{
    load_replay_bundle_manifest, replay_bundle, replay_bundle_by_id, ReplayError, ReplayReport,
};
pub use state_machine::{
    can_transition_artifact_lifecycle, can_transition_freshness,
    compute_default_artifact_admissibility, compute_default_packet_admissibility,
    validate_artifact_state, validate_packet_state,
};
pub mod authorization_evidence;
pub mod consumer_acknowledgment;
pub mod consumer_handoff;
pub mod downstream_release;
pub mod import_authorization;
pub mod import_contract;
pub mod import_gate;
pub mod import_policy;
pub mod lineage_activation;
pub mod lineage_bundle;
pub mod lineage_bundle_intake;
pub mod lineage_bundle_rehydrate;
pub mod lineage_consumption;
pub mod promotion_gate;
pub mod promotion_revocation;
pub mod re_promotion;
pub mod release_attestation;
pub mod release_readiness;
pub mod sealed_release_bundle;
pub mod supersession_chain;
pub mod terminal_consumer_import;
pub mod trust_envelope;
