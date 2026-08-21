//! Phase-1 governed context assembly for the first proofread contract.
//!
//! This module is intentionally self-contained for the first bounded slice.
//! It can be proved in isolation before crate-root export and schema-bundle
//! registration are wired into the live repo.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Identity prefix for the current scheme. Carries the algorithm so a resolver
/// never has to infer it from length.
pub const ID_PREFIX: &str = "ctxb.sha256.";

/// Identity prefix for everything minted before Slice 39. Untagged, because at
/// the time there was only one algorithm to be.
pub const LEGACY_ID_PREFIX: &str = "ctxb_";

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub enum SourceClass {
    ActiveScene,
    AdjacentSceneSummaryOrClippedBody,
    AcceptedLoreRecord,
    AcceptedStyleRuleRecord,
    // A fact that passed forge-memory's governance.
    //
    // Named for what it is rather than where it came from, matching the
    // `Accepted*Record` style: "memory source" would name the pipe.
    //
    // Admission is still double-gated — a source must be phase-1 allowed *and*
    // listed in the request's `allowed_source_classes` — so this class becomes
    // available, never supplied. No manuscript profile that predates it can
    // receive one by accident.
    //
    // Deliberately a line comment rather than a doc comment: schemars promotes an
    // enum whose variants carry docs from a flat `enum` list into a `oneOf` of
    // per-variant branches. That is semantically equivalent and structurally
    // different, and a consumer reading the exported schema would see the shape
    // change rather than one more string in a list. The explanation belongs to
    // readers of this file; the schema should record only that the class exists.
    GovernedMemoryFact,
    ExperimentalFutureSource,
}

impl SourceClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ActiveScene => "active_scene",
            Self::AdjacentSceneSummaryOrClippedBody => "adjacent_scene_summary_or_clipped_body",
            Self::AcceptedLoreRecord => "accepted_lore_record",
            Self::AcceptedStyleRuleRecord => "accepted_style_rule_record",
            Self::GovernedMemoryFact => "governed_memory_fact",
            Self::ExperimentalFutureSource => "experimental_future_source",
        }
    }

    pub fn is_phase1_allowed(&self) -> bool {
        matches!(
            self,
            Self::ActiveScene
                | Self::AdjacentSceneSummaryOrClippedBody
                | Self::AcceptedLoreRecord
                | Self::AcceptedStyleRuleRecord
                | Self::GovernedMemoryFact
        )
    }

    /// Whether a source of this class must carry [`SourceProvenance`].
    ///
    /// Only governed memory does. A manuscript source is identified by the
    /// target refs the request already names; a memory fact is not, so without
    /// provenance a bundle could not say which fact it rested on or under what
    /// authority it was used.
    pub fn requires_provenance(&self) -> bool {
        matches!(self, Self::GovernedMemoryFact)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OverridePosture {
    DisallowAll,
    AllowAcceptedStyleRuleRecords,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OverrideDecision {
    NoOverridePresent,
    AllowedStyleRuleOverrideUsed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AuthorityState {
    Accepted,
    ConflictResolved,
    ConflictUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FreshnessBand {
    Fresh,
    NearLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ReplayEligibility {
    Eligible,
}

/// One class's own staleness limit, overriding the bundle-wide default.
///
/// A list rather than a map, deliberately: a map's iteration order is not
/// stable, and this repository binds identity to serialized form in enough
/// places that an unordered collection is a latent replay hazard even where it
/// is not hashed today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClassFreshnessOverride {
    pub source_class: SourceClass,
    pub max_source_age_minutes: u64,
}

/// How old a source may be before it is refused.
///
/// `max_source_age_minutes` governs every class that has no override, and by
/// itself it is the whole policy — which was the problem. Source classes have
/// different natural lifetimes: an active scene is stale in minutes, and a
/// governed memory fact earns its value by persisting. One number set for scenes
/// refuses every memory fact; set for memory it admits a stale scene.
///
/// `class_overrides` is `Option` and skipped when absent, so a policy that never
/// mentions it serializes byte-identically to one written before this field
/// existed, and behaves identically too. Nothing that assembles today changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FreshnessPolicy {
    pub max_source_age_minutes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_overrides: Option<Vec<ClassFreshnessOverride>>,
}

impl FreshnessPolicy {
    /// A policy with no per-class overrides — the shape every caller had before
    /// they existed.
    pub fn uniform(max_source_age_minutes: u64) -> Self {
        FreshnessPolicy {
            max_source_age_minutes,
            class_overrides: None,
        }
    }

    /// The staleness limit that applies to `source_class`.
    pub fn max_for(&self, source_class: &SourceClass) -> u64 {
        self.class_overrides
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|entry| &entry.source_class == source_class)
            .map(|entry| entry.max_source_age_minutes)
            .unwrap_or(self.max_source_age_minutes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetRefs {
    pub active_scene_ref: Option<String>,
    pub adjacent_scene_ref: Option<String>,
    pub accepted_lore_record_refs: Vec<String>,
    pub accepted_style_rule_refs: Vec<String>,
}

/// Where a governed memory source came from and under what authority.
///
/// Every field is a reference. Nothing here carries memory content: this
/// records *which* fact a bundle rested on, not what the fact said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceProvenance {
    /// The memory fact, in forge_contract_core reference grammar:
    /// `<artifact_family>:<artifact_id>:v<artifact_version>`.
    pub memory_ref: String,
    /// The `memory_retrieval_receipt` that supplied it.
    pub retrieval_receipt_ref: String,
    /// What authorized its use here — typically a `memory_residency_policy`
    /// reference. Recorded, never authenticated here.
    pub authority_ref: String,
}

impl SourceProvenance {
    /// The hash contribution for this provenance.
    ///
    /// Kept beside the struct so the entry piece in [`compute_bundle_hash`] and
    /// the fields cannot drift apart: adding a field here without extending this
    /// would leave it recorded but unbound, which is a label rather than
    /// evidence.
    fn hash_piece(&self) -> String {
        format!(
            "|{}|{}|{}",
            self.memory_ref, self.retrieval_receipt_ref, self.authority_ref
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceInput {
    pub payload_ref: String,
    pub source_class: SourceClass,
    pub age_minutes: u64,
    pub authority_state: AuthorityState,
    pub is_override: bool,
    /// Required for [`SourceClass::GovernedMemoryFact`], absent otherwise.
    ///
    /// Optional in the type so every existing caller keeps compiling and every
    /// existing serialized request keeps deserializing; mandatory in the rule so
    /// a memory fact that cannot say where it came from is refused rather than
    /// admitted unlabelled. Optional alone would not be fail-closed, and
    /// required alone would break every manuscript caller. Neither is enough by
    /// itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<SourceProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContextAssemblyRequest {
    pub task_intent_id: String,
    pub task_family: String,
    pub task_version: String,
    pub target_refs: TargetRefs,
    pub allowed_source_classes: Vec<SourceClass>,
    pub freshness_policy: FreshnessPolicy,
    pub override_posture: OverridePosture,
    pub sources: Vec<SourceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceInventoryEntry {
    pub payload_ref: String,
    pub source_class: SourceClass,
    pub age_minutes: u64,
    pub authority_state: AuthorityState,
    pub is_override: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<SourceProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContextBundleManifest {
    /// Algorithm-tagged: `ctxb.sha256.<64 hex>`.
    ///
    /// The tag is the point. An untagged identity cannot say which function
    /// produced it, so a resolver holding a mixed population has to guess — and
    /// the guess is only safe while exactly one algorithm has ever been used,
    /// which is the assumption this slice exists to remove.
    pub context_bundle_id: String,
    /// SHA-256 over the same canonical string the legacy digest covers.
    pub bundle_hash: String,
    /// The FNV-1a 64 identity this bundle would have had before Slice 39, and
    /// still has wherever it was already recorded.
    ///
    /// Not deprecated dead weight: `context_packs` in DataForge keys on the id
    /// and has no retention — no `DELETE`, no TTL, no cleanup job — so rows
    /// minted under the old scheme never age out. Resolution across both forms
    /// is therefore permanent rather than transitional, and a consumer needs
    /// this value to find a pack stored before the change.
    pub legacy_context_bundle_id: String,
    pub legacy_bundle_hash: String,
    pub source_inventory: Vec<SourceInventoryEntry>,
    pub freshness_band: FreshnessBand,
    pub override_decision: OverrideDecision,
    pub authority_conflict_flag: bool,
    pub replay_eligibility: ReplayEligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContextAssemblyOutput {
    pub manifest: ContextBundleManifest,
    pub payload_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAssemblyError {
    MissingRequiredSource {
        payload_ref: String,
        source_class: SourceClass,
    },
    StaleSource {
        payload_ref: String,
        age_minutes: u64,
        max_age_minutes: u64,
    },
    AuthorityConflictUnresolved {
        payload_ref: String,
    },
    DisallowedOverride {
        payload_ref: String,
        source_class: SourceClass,
    },
    UnsupportedSourceClass {
        source_class: SourceClass,
    },
    MissingProvenance {
        payload_ref: String,
        source_class: SourceClass,
    },
    DuplicateFreshnessOverride {
        source_class: SourceClass,
    },
    UnsupportedFreshnessOverride {
        source_class: SourceClass,
    },
}

impl fmt::Display for ContextAssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredSource { payload_ref, source_class } => {
                write!(f, "missing required source {} ({})", payload_ref, source_class.as_str())
            }
            Self::StaleSource {
                payload_ref,
                age_minutes,
                max_age_minutes,
            } => write!(
                f,
                "stale source {} (age {}m > max {}m)",
                payload_ref, age_minutes, max_age_minutes
            ),
            Self::AuthorityConflictUnresolved { payload_ref } => {
                write!(f, "authority conflict unresolved for {}", payload_ref)
            }
            Self::DisallowedOverride {
                payload_ref,
                source_class,
            } => write!(
                f,
                "disallowed override for {} ({})",
                payload_ref,
                source_class.as_str()
            ),
            Self::UnsupportedSourceClass { source_class } => {
                write!(f, "unsupported source class {}", source_class.as_str())
            }
            Self::DuplicateFreshnessOverride { source_class } => write!(
                f,
                "duplicate freshness override for {}: one class cannot have two limits",
                source_class.as_str()
            ),
            Self::UnsupportedFreshnessOverride { source_class } => write!(
                f,
                "freshness override for unsupported source class {}",
                source_class.as_str()
            ),
            Self::MissingProvenance {
                payload_ref,
                source_class,
            } => write!(
                f,
                "missing provenance for {} ({}): a {} source must name the memory it came from, the receipt that supplied it, and the authority it was used under",
                payload_ref,
                source_class.as_str(),
                source_class.as_str()
            ),
        }
    }
}

impl std::error::Error for ContextAssemblyError {}

pub fn assemble_context(
    request: &ContextAssemblyRequest,
) -> Result<ContextAssemblyOutput, ContextAssemblyError> {
    validate_allowed_classes(&request.allowed_source_classes)?;
    validate_freshness_policy(&request.freshness_policy)?;
    validate_required_target_refs(request)?;

    let mut inventory: Vec<SourceInventoryEntry> = Vec::with_capacity(request.sources.len());
    let mut payload_refs: Vec<String> = Vec::with_capacity(request.sources.len());
    let mut saw_source_near_limit = false;
    let mut saw_conflict_resolution = false;
    let mut saw_allowed_override = false;

    for source in &request.sources {
        if !source.source_class.is_phase1_allowed()
            || !request
                .allowed_source_classes
                .contains(&source.source_class)
        {
            return Err(ContextAssemblyError::UnsupportedSourceClass {
                source_class: source.source_class.clone(),
            });
        }

        if source.source_class.requires_provenance() && source.provenance.is_none() {
            return Err(ContextAssemblyError::MissingProvenance {
                payload_ref: source.payload_ref.clone(),
                source_class: source.source_class.clone(),
            });
        }

        // The limit that applies to *this* class, which is the bundle-wide one
        // unless the policy said otherwise. StaleSource reports the effective
        // limit rather than the default, so the error names the rule that
        // actually refused the source.
        let max_age_minutes = request.freshness_policy.max_for(&source.source_class);
        if source.age_minutes > max_age_minutes {
            return Err(ContextAssemblyError::StaleSource {
                payload_ref: source.payload_ref.clone(),
                age_minutes: source.age_minutes,
                max_age_minutes,
            });
        }

        // A source is near its limit at half of it or more. Judged per source
        // against its own limit, because with per-class limits the oldest source
        // is no longer necessarily the closest to being refused.
        //
        // A limit of zero contributes nothing: such a source must be zero
        // minutes old to be here at all, and calling that "near limit" was not
        // what the bundle-wide rule did either.
        if max_age_minutes > 0 && source.age_minutes * 2 >= max_age_minutes {
            saw_source_near_limit = true;
        }

        if source.is_override {
            let allowed = matches!(
                (&request.override_posture, &source.source_class),
                (
                    OverridePosture::AllowAcceptedStyleRuleRecords,
                    SourceClass::AcceptedStyleRuleRecord
                )
            );

            if !allowed {
                return Err(ContextAssemblyError::DisallowedOverride {
                    payload_ref: source.payload_ref.clone(),
                    source_class: source.source_class.clone(),
                });
            }

            saw_allowed_override = true;
        }

        if matches!(source.authority_state, AuthorityState::ConflictUnresolved) {
            return Err(ContextAssemblyError::AuthorityConflictUnresolved {
                payload_ref: source.payload_ref.clone(),
            });
        }

        if matches!(source.authority_state, AuthorityState::ConflictResolved) {
            saw_conflict_resolution = true;
        }

        payload_refs.push(source.payload_ref.clone());
        inventory.push(SourceInventoryEntry {
            payload_ref: source.payload_ref.clone(),
            source_class: source.source_class.clone(),
            age_minutes: source.age_minutes,
            authority_state: source.authority_state.clone(),
            is_override: source.is_override,
            provenance: source.provenance.clone(),
        });
    }

    inventory.sort_by(|left, right| {
        left.source_class
            .cmp(&right.source_class)
            .then(left.payload_ref.cmp(&right.payload_ref))
    });
    payload_refs.sort();

    // Equivalent to the bundle-wide rule it replaces whenever no override is
    // present: with a single limit L, `max(age) * 2 >= L` holds exactly when some
    // source satisfies `age * 2 >= L`, and the L == 0 case is carried by the
    // per-source guard above. Every existing bundle keeps its band.
    let freshness_band = if saw_source_near_limit {
        FreshnessBand::NearLimit
    } else {
        FreshnessBand::Fresh
    };

    let override_decision = if saw_allowed_override {
        OverrideDecision::AllowedStyleRuleOverrideUsed
    } else {
        OverrideDecision::NoOverridePresent
    };

    let canonical = canonical_string(request, &inventory);
    let bundle_hash = strong_digest(&canonical);
    let context_bundle_id = format!("{ID_PREFIX}{bundle_hash}");
    let legacy_bundle_hash = legacy_digest(&canonical);
    let legacy_context_bundle_id = format!("{LEGACY_ID_PREFIX}{legacy_bundle_hash}");

    Ok(ContextAssemblyOutput {
        manifest: ContextBundleManifest {
            context_bundle_id,
            bundle_hash,
            legacy_context_bundle_id,
            legacy_bundle_hash,
            source_inventory: inventory,
            freshness_band,
            override_decision,
            authority_conflict_flag: saw_conflict_resolution,
            replay_eligibility: ReplayEligibility::Eligible,
        },
        payload_refs,
    })
}

fn validate_allowed_classes(
    allowed_source_classes: &[SourceClass],
) -> Result<(), ContextAssemblyError> {
    for source_class in allowed_source_classes {
        if !source_class.is_phase1_allowed() {
            return Err(ContextAssemblyError::UnsupportedSourceClass {
                source_class: source_class.clone(),
            });
        }
    }
    Ok(())
}

/// A policy may not name one class twice, or name a class that cannot be used.
///
/// Both are refused rather than ignored. A duplicate means two rules for one
/// class and no stated way to choose between them; an override for an
/// unsupported class is dead configuration, which is what a typo looks like.
fn validate_freshness_policy(policy: &FreshnessPolicy) -> Result<(), ContextAssemblyError> {
    let Some(overrides) = policy.class_overrides.as_deref() else {
        return Ok(());
    };
    let mut seen: Vec<&SourceClass> = Vec::with_capacity(overrides.len());
    for entry in overrides {
        if !entry.source_class.is_phase1_allowed() {
            return Err(ContextAssemblyError::UnsupportedFreshnessOverride {
                source_class: entry.source_class.clone(),
            });
        }
        if seen.contains(&&entry.source_class) {
            return Err(ContextAssemblyError::DuplicateFreshnessOverride {
                source_class: entry.source_class.clone(),
            });
        }
        seen.push(&entry.source_class);
    }
    Ok(())
}

fn validate_required_target_refs(
    request: &ContextAssemblyRequest,
) -> Result<(), ContextAssemblyError> {
    require_target_ref(
        request,
        request.target_refs.active_scene_ref.as_ref(),
        SourceClass::ActiveScene,
    )?;
    require_target_ref(
        request,
        request.target_refs.adjacent_scene_ref.as_ref(),
        SourceClass::AdjacentSceneSummaryOrClippedBody,
    )?;

    for payload_ref in &request.target_refs.accepted_lore_record_refs {
        require_target_ref(request, Some(payload_ref), SourceClass::AcceptedLoreRecord)?;
    }

    for payload_ref in &request.target_refs.accepted_style_rule_refs {
        require_target_ref(
            request,
            Some(payload_ref),
            SourceClass::AcceptedStyleRuleRecord,
        )?;
    }

    Ok(())
}

fn require_target_ref(
    request: &ContextAssemblyRequest,
    payload_ref: Option<&String>,
    expected_class: SourceClass,
) -> Result<(), ContextAssemblyError> {
    if let Some(payload_ref) = payload_ref {
        let present = request.sources.iter().any(|source| {
            source.payload_ref == *payload_ref && source.source_class == expected_class
        });

        if !present {
            return Err(ContextAssemblyError::MissingRequiredSource {
                payload_ref: payload_ref.clone(),
                source_class: expected_class,
            });
        }
    }

    Ok(())
}

/// The canonical string both digests are taken over.
///
/// Extracted unchanged from the previous `compute_bundle_hash`. Keeping one
/// canonicalisation for both algorithms is what makes the legacy digest provably
/// identical to what it was: the bytes fed to FNV are the same bytes, so a
/// legacy id computed now equals the one recorded then.
fn canonical_string(
    request: &ContextAssemblyRequest,
    inventory: &[SourceInventoryEntry],
) -> String {
    let mut pieces = vec![
        request.task_intent_id.clone(),
        request.task_family.clone(),
        request.task_version.clone(),
    ];

    // Provenance joins the piece only when present, so an entry without it
    // hashes to the byte-identical string it always did. Every bundle assembled
    // before this field existed keeps its bundle_hash and context_bundle_id.
    //
    // When present it is bound into the bundle's identity rather than merely
    // recorded alongside it: a bundle cannot silently change which memory fact
    // it rested on, or which receipt supplied it, while keeping its id.
    for entry in inventory {
        let mut piece = format!(
            "{}|{}|{}|{:?}|{}",
            entry.payload_ref,
            entry.source_class.as_str(),
            entry.age_minutes,
            entry.authority_state,
            entry.is_override
        );
        if let Some(provenance) = &entry.provenance {
            piece.push_str(&provenance.hash_piece());
        }
        pieces.push(piece);
    }

    pieces.join("||")
}

/// The pre-Slice-39 identity. Retained because recorded ids must stay
/// resolvable, not because the digest is fit for the job it was given.
fn legacy_digest(canonical: &str) -> String {
    format!("{:016x}", fnv1a64(canonical.as_bytes()))
}

/// The identity this repository mints now.
///
/// FNV-1a is a fast non-cryptographic hash, and 64 bits of it is 2^32 birthday
/// work — seconds of ordinary hardware. That was an unremarkable choice while
/// the value was a deterministic name. It stopped being one when PACT's packet
/// builders and forgeHQ's verification bridge began treating the value as an
/// integrity binding, the latter describing it as proof that a verification "was
/// produced against the exact governed, replay-eligible context". A binding that
/// cheap to forge does not carry that claim.
fn strong_digest(canonical: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
