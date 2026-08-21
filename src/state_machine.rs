use crate::enums::{
    AdmissibilityState, CriticStatus, FreshnessState, LifecycleState, PacketLifecycleState,
};
use crate::models::{ArtifactRecord, PacketRecord};

pub fn compute_default_artifact_admissibility(
    lifecycle_state: LifecycleState,
    freshness_state: FreshnessState,
    critic_status: CriticStatus,
) -> AdmissibilityState {
    match lifecycle_state {
        LifecycleState::Candidate
        | LifecycleState::Blocked
        | LifecycleState::Superseded
        | LifecycleState::Retired => AdmissibilityState::NotAdmissible,
        LifecycleState::Approved => match critic_status {
            CriticStatus::Failed | CriticStatus::RemediationRequired => {
                AdmissibilityState::NotAdmissible
            }
            CriticStatus::NotReviewed => AdmissibilityState::Restricted,
            CriticStatus::Passed => match freshness_state {
                FreshnessState::Fresh => AdmissibilityState::Admissible,
                FreshnessState::ReviewDue => AdmissibilityState::AdmissibleWithWarning,
                FreshnessState::Stale => AdmissibilityState::Restricted,
                FreshnessState::Invalidated => AdmissibilityState::NotAdmissible,
            },
            CriticStatus::PassedWithConcerns => match freshness_state {
                FreshnessState::Fresh => AdmissibilityState::Restricted,
                FreshnessState::ReviewDue => AdmissibilityState::Restricted,
                FreshnessState::Stale => AdmissibilityState::NotAdmissible,
                FreshnessState::Invalidated => AdmissibilityState::NotAdmissible,
            },
        },
    }
}

pub fn compute_default_packet_admissibility(
    lifecycle_state: PacketLifecycleState,
    required_constituents_present: bool,
    reevaluation_required: bool,
    constituent_freshnesses: &[FreshnessState],
    constituent_lifecycles: &[LifecycleState],
) -> AdmissibilityState {
    if lifecycle_state != PacketLifecycleState::Approved {
        return AdmissibilityState::NotAdmissible;
    }

    if !required_constituents_present || reevaluation_required {
        return AdmissibilityState::NotAdmissible;
    }

    if constituent_lifecycles.iter().any(|state| {
        matches!(
            state,
            LifecycleState::Candidate
                | LifecycleState::Blocked
                | LifecycleState::Superseded
                | LifecycleState::Retired
        )
    }) {
        return AdmissibilityState::NotAdmissible;
    }

    if constituent_freshnesses.contains(&FreshnessState::Invalidated) {
        return AdmissibilityState::NotAdmissible;
    }

    if constituent_freshnesses.contains(&FreshnessState::Stale) {
        return AdmissibilityState::Restricted;
    }

    if constituent_freshnesses.contains(&FreshnessState::ReviewDue) {
        return AdmissibilityState::AdmissibleWithWarning;
    }

    AdmissibilityState::Admissible
}

pub fn validate_artifact_state(artifact: &ArtifactRecord) -> Result<(), String> {
    if artifact.schema_version.trim().is_empty() {
        return Err("artifact schema_version is required".into());
    }
    if artifact.artifact_id.trim().is_empty() {
        return Err("artifact_id is required".into());
    }
    if artifact.repo_id.trim().is_empty() {
        return Err("repo_id is required".into());
    }
    if artifact.title.trim().is_empty() {
        return Err("title is required".into());
    }
    if artifact.source_refs.is_empty() {
        return Err("source_refs must not be empty".into());
    }
    if artifact.source_refs.len() != artifact.source_ref_hashes.len() {
        return Err("source_refs and source_ref_hashes must stay aligned".into());
    }

    // Specific illegal combinations first so tests and operator diagnostics stay precise.
    if artifact.lifecycle_state == LifecycleState::Candidate
        && artifact.admissibility_state == AdmissibilityState::Admissible
    {
        return Err("candidate artifact cannot be admissible".into());
    }

    if artifact.lifecycle_state == LifecycleState::Candidate
        && artifact.freshness_state == FreshnessState::Fresh
    {
        return Err("candidate artifact cannot be fresh".into());
    }

    if artifact.lifecycle_state == LifecycleState::Blocked
        && artifact.freshness_state == FreshnessState::Fresh
    {
        return Err("blocked artifact cannot remain fresh".into());
    }

    if artifact.lifecycle_state == LifecycleState::Blocked
        && artifact.admissibility_state == AdmissibilityState::AdmissibleWithWarning
    {
        return Err("blocked artifact cannot be admissible_with_warning".into());
    }

    if artifact.lifecycle_state == LifecycleState::Superseded
        && artifact.admissibility_state == AdmissibilityState::Restricted
    {
        return Err("superseded artifact cannot be restricted".into());
    }

    if artifact.lifecycle_state == LifecycleState::Retired
        && artifact.admissibility_state != AdmissibilityState::NotAdmissible
    {
        return Err("retired artifact must be not_admissible".into());
    }

    if artifact.critic_status == CriticStatus::Failed
        && artifact.lifecycle_state == LifecycleState::Approved
    {
        return Err("critic failed artifact cannot remain approved".into());
    }

    if artifact.critic_status == CriticStatus::Failed
        && artifact.admissibility_state != AdmissibilityState::NotAdmissible
    {
        return Err("critic failed artifact cannot remain admissible".into());
    }

    if artifact.freshness_state == FreshnessState::Invalidated
        && artifact.admissibility_state != AdmissibilityState::NotAdmissible
    {
        return Err("invalidated artifact cannot remain admissible".into());
    }

    let expected = compute_default_artifact_admissibility(
        artifact.lifecycle_state,
        artifact.freshness_state,
        artifact.critic_status,
    );

    if artifact.admissibility_state != expected {
        return Err(format!(
            "artifact admissibility_state does not match canonical state algebra: expected {:?}, got {:?}",
            expected, artifact.admissibility_state
        ));
    }

    Ok(())
}

pub fn validate_packet_state(packet: &PacketRecord) -> Result<(), String> {
    if packet.schema_version.trim().is_empty() {
        return Err("packet schema_version is required".into());
    }
    if packet.packet_id.trim().is_empty() {
        return Err("packet_id is required".into());
    }
    if packet.repo_id.trim().is_empty() {
        return Err("packet repo_id is required".into());
    }
    if packet.included_artifact_ids.is_empty() {
        return Err("included_artifact_ids must not be empty".into());
    }
    if packet.included_artifact_ids.len() != packet.included_artifact_hashes.len() {
        return Err("included_artifact_ids and included_artifact_hashes must stay aligned".into());
    }

    if packet.lifecycle_state == PacketLifecycleState::Candidate
        && packet.admissibility_state == AdmissibilityState::Admissible
    {
        return Err("candidate packet cannot be admissible".into());
    }

    if packet.lifecycle_state == PacketLifecycleState::Approved
        && !packet.required_constituents_present
    {
        return Err("approved packet cannot exist with missing required constituents".into());
    }

    if packet.lifecycle_state == PacketLifecycleState::Approved && packet.reevaluation_required {
        return Err("approved packet cannot exist while reevaluation is required".into());
    }

    Ok(())
}

pub fn can_transition_artifact_lifecycle(from: LifecycleState, to: LifecycleState) -> bool {
    matches!(
        (from, to),
        (LifecycleState::Candidate, LifecycleState::Approved)
            | (LifecycleState::Candidate, LifecycleState::Blocked)
            | (LifecycleState::Approved, LifecycleState::Superseded)
            | (LifecycleState::Approved, LifecycleState::Blocked)
            | (LifecycleState::Blocked, LifecycleState::Candidate)
            | (LifecycleState::Superseded, LifecycleState::Retired)
    )
}

pub fn can_transition_freshness(from: FreshnessState, to: FreshnessState) -> bool {
    matches!(
        (from, to),
        (FreshnessState::Fresh, FreshnessState::ReviewDue)
            | (FreshnessState::ReviewDue, FreshnessState::Stale)
            | (FreshnessState::Fresh, FreshnessState::Invalidated)
            | (FreshnessState::ReviewDue, FreshnessState::Invalidated)
            | (FreshnessState::Stale, FreshnessState::Invalidated)
            | (FreshnessState::Stale, FreshnessState::Fresh)
            | (FreshnessState::Invalidated, FreshnessState::Fresh)
    )
}
