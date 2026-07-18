use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt};
use uuid::Uuid;

pub const EVALUATION_PROTOCOL_VERSION: u16 = 1;
pub const MAX_ACTOR_DISPLAY_NAME_BYTES: usize = 256;
pub const MAX_ACTOR_VERSION_BYTES: usize = 128;
pub const MAX_CLIENT_NAME_BYTES: usize = 128;
pub const MAX_SESSION_ID_BYTES: usize = 256;
pub const MAX_SOURCE_MEETING_ID_BYTES: usize = 256;
pub const MAX_REASON_BYTES: usize = 4096;
pub const MAX_MEETING_EVIDENCE_BYTES: usize = 65_536;
pub const MAX_RELEVANT_FIELDS: usize = 32;

macro_rules! evaluation_id {
    ($name:ident,$prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn new() -> Self {
                Self(format!(concat!($prefix, "_{}"), Uuid::new_v4().simple()))
            }
            pub fn from_string(value: impl Into<String>) -> Result<Self, RowvaError> {
                let value = value.into();
                if value.starts_with(concat!($prefix, "_")) && value.len() > 4 {
                    Ok(Self(value))
                } else {
                    Err(RowvaError::validation(
                        "invalid_id",
                        concat!("invalid ", $prefix, " identifier"),
                    ))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}
evaluation_id!(EvaluationCaseId, "evc");
evaluation_id!(EvaluationCandidateId, "evn");
evaluation_id!(EvaluationResultId, "evr");
evaluation_id!(EvaluationInvalidationId, "evi");
evaluation_id!(EvaluationDatasetId, "evd");
evaluation_id!(EvaluationAnalysisRunId, "eva");

pub const EVALUATION_DATASET_PROTOCOL_VERSION: u16 = 1;
pub const EVALUATION_QUALITY_REVISION: u16 = 1;
pub const EVALUATION_CALIBRATION_REVISION: u16 = 1;
pub const EVALUATION_SCORER_REVISION: u16 = 1;
pub const EVALUATION_READINESS_REVISION: u16 = 1;
pub const MAX_DATASET_CASES: usize = 10_000;
pub const MAX_DATASET_NAME_BYTES: usize = 256;
pub const MAX_DATASET_DESCRIPTION_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationWorkflow {
    DealStageQualificationV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationCaseStatus {
    CollectingCandidates,
    OutcomeRecorded,
    Scored,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DealStageQualificationInputV1 {
    pub source_meeting_id: Option<String>,
    pub meeting_evidence: Value,
    pub current_stage: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCaseCreate {
    pub protocol_version: u16,
    pub target_object_id: ObjectId,
    pub target_record_id: RecordId,
    pub target_stage_field_id: FieldId,
    pub relevant_field_ids: Vec<FieldId>,
    pub input: DealStageQualificationInputV1,
    pub creator: ActorContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCaseBundleV1 {
    pub protocol_version: u16,
    pub case_id: EvaluationCaseId,
    pub workspace_id: WorkspaceId,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub target_object_id: ObjectId,
    pub target_record_id: RecordId,
    pub target_stage_field: FieldDefinition,
    pub base_schema_revision: SchemaRevision,
    pub base_record_revision: RecordRevision,
    pub record_snapshot: HashMap<FieldId, Value>,
    pub current_stage: Value,
    pub input: DealStageQualificationInputV1,
    pub permitted_output_schema: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCaseExport {
    pub bundle: EvaluationCaseBundleV1,
    pub bundle_digest: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationCase {
    pub bundle: EvaluationCaseBundleV1,
    pub id: EvaluationCaseId,
    pub workspace_id: WorkspaceId,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub target_object_id: ObjectId,
    pub target_record_id: RecordId,
    pub target_stage_field: FieldDefinition,
    pub base_schema_revision: SchemaRevision,
    pub base_record_revision: RecordRevision,
    pub record_snapshot: HashMap<FieldId, Value>,
    pub current_stage: Value,
    pub input: DealStageQualificationInputV1,
    pub bundle_digest: String,
    pub evidence_digest: String,
    pub creator: ActorContext,
    pub created_at: DateTime<Utc>,
    pub status: EvaluationCaseStatus,
    pub invalidated_at: Option<DateTime<Utc>>,
    pub invalidation_reason: Option<String>,
}
impl EvaluationCase {
    pub fn id(&self) -> &EvaluationCaseId {
        &self.bundle.case_id
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageUpdateProposalV1 {
    pub object_id: ObjectId,
    pub record_id: RecordId,
    pub field_id: FieldId,
    pub expected_revision: RecordRevision,
    pub value: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShadowDecision {
    ProposeStageUpdate { proposal: StageUpdateProposalV1 },
    NoChange { reason: Option<String> },
    Abstain { reason: String },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateImport {
    pub protocol_version: u16,
    pub case_id: EvaluationCaseId,
    pub bundle_digest: String,
    pub actor: ActorContext,
    pub decision: ShadowDecision,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateValidationStatus {
    Valid,
    InvalidTarget,
    StaleRevision,
    InvalidValue,
    UnsafeExtraChanges,
    Invalid,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationCandidate {
    pub id: EvaluationCandidateId,
    pub case_id: EvaluationCaseId,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub bundle_digest: String,
    pub actor: ActorContext,
    pub actor_version: String,
    pub decision: ShadowDecision,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub fingerprint: String,
    pub submitted_at: DateTime<Utc>,
    pub validation_status: CandidateValidationStatus,
    pub validation_error: Option<RowvaError>,
    pub eligible_for_metrics: bool,
    pub attempt_number: u32,
    pub eligibility_reason: String,
    pub supersedes_candidate_id: Option<EvaluationCandidateId>,
    pub normalized_changes: Vec<ProposedChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HumanEvaluationOutcomeInput {
    CommittedOperation {
        operation_id: OperationId,
    },
    NoChange {
        actor: ActorContext,
        reason: Option<String>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanEvaluationOutcome {
    pub case_id: EvaluationCaseId,
    pub outcome: HumanEvaluationOutcomeInput,
    pub normalized_stage: Value,
    pub recorded_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationVerdict {
    ExactAgreement,
    NoChangeAgreement,
    FalsePositive,
    FalseNegative,
    WrongStage,
    Abstained,
    InvalidTarget,
    StaleRevision,
    InvalidValue,
    UnsafeExtraChanges,
    InvalidCandidate,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub id: EvaluationResultId,
    pub case_id: EvaluationCaseId,
    pub candidate_id: EvaluationCandidateId,
    pub scorer_revision: u16,
    pub candidate_stage: Option<Value>,
    pub human_stage: Value,
    pub verdict: EvaluationVerdict,
    pub eligible: bool,
    pub scored_at: DateTime<Utc>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenReplayResult {
    pub replay_version: u16,
    pub kind: String,
    pub case_id: EvaluationCaseId,
    pub candidate_id: EvaluationCandidateId,
    pub bundle_digest: String,
    pub validation_status: CandidateValidationStatus,
    pub changes: Vec<ProposedChange>,
    pub error: Option<RowvaError>,
    pub deterministic_match: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessClass {
    InsufficientEvidence,
    NotReady,
    Promising,
    CandidateForHumanReview,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvaluationCounts {
    pub cases_available: u64,
    pub distinct_cases_attempted: u64,
    pub total_submitted_attempts: u64,
    pub eligible_decisions: u64,
    pub ineligible_retries: u64,
    pub pending_unscored_eligible: u64,
    pub scored_eligible: u64,
    pub valid_scored: u64,
    pub invalid_scored: u64,
    pub exact_change_agreements: u64,
    pub no_change_agreements: u64,
    pub agreement_count: u64,
    pub false_positives: u64,
    pub false_negatives: u64,
    pub wrong_stage: u64,
    pub abstentions: u64,
    pub exclusions_by_reason: HashMap<String, u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub workflow: EvaluationWorkflow,
    pub actor_id: ActorId,
    pub actor_version: String,
    pub counts: EvaluationCounts,
    pub agreement_rate: Option<f64>,
    pub exact_change_agreement_rate: Option<f64>,
    pub coverage_rate: Option<f64>,
    pub invalid_proposal_rate: Option<f64>,
    pub readiness_rubric_revision: u16,
    pub readiness: ReadinessClass,
    pub readiness_reasons: Vec<String>,
    pub advisory_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationInvalidationCategory {
    IncorrectInput,
    DuplicateCase,
    OutcomeLeakage,
    AmbiguousHumanReference,
    WrongSchemaOrTarget,
    IntegrityFailure,
    PrivacyOrRetentionRequest,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationInvalidationInput {
    pub protocol_version: u16,
    pub case_id: EvaluationCaseId,
    pub category: EvaluationInvalidationCategory,
    pub reason: String,
    pub actor: ActorContext,
    pub related_case_id: Option<EvaluationCaseId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationInvalidation {
    pub id: EvaluationInvalidationId,
    pub case_id: EvaluationCaseId,
    pub category: EvaluationInvalidationCategory,
    pub reason: String,
    pub actor: ActorContext,
    pub previous_status: EvaluationCaseStatus,
    pub invalidated_at: DateTime<Utc>,
    pub case_bundle_digest: String,
    pub related_case_id: Option<EvaluationCaseId>,
    pub protocol_version: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvaluationDatasetSelectionV1 {
    Explicit {
        case_ids: Vec<EvaluationCaseId>,
    },
    AllScored {
        created_at_or_after: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationDatasetCreateV1 {
    pub protocol_version: u16,
    pub name: String,
    pub description: Option<String>,
    pub selection: EvaluationDatasetSelectionV1,
    pub creator: ActorContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationDatasetCaseMemberV1 {
    pub case_id: EvaluationCaseId,
    pub case_bundle_digest: String,
    pub status_at_snapshot: EvaluationCaseStatus,
    pub case_created_at: DateTime<Utc>,
    pub human_outcome_present: bool,
    pub scorer_revisions: Vec<u16>,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationDatasetManifestV1 {
    pub protocol_version: u16,
    pub dataset_id: EvaluationDatasetId,
    pub name: String,
    pub description: Option<String>,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub selection: EvaluationDatasetSelectionV1,
    pub case_members: Vec<EvaluationDatasetCaseMemberV1>,
    pub creator: ActorContext,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationDataset {
    pub manifest: EvaluationDatasetManifestV1,
    pub dataset_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationAnalysisRequestV1 {
    pub protocol_version: u16,
    pub dataset_id: EvaluationDatasetId,
    pub actor_id: ActorId,
    pub actor_version: String,
    pub scorer_revision: u16,
    pub readiness_rubric_revision: u16,
    pub quality_revision: u16,
    pub calibration_revision: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationQualityWarning {
    pub code: String,
    pub case_ids: Vec<EvaluationCaseId>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvaluationQualityReportV1 {
    pub revision: u16,
    pub dataset_members: u64,
    pub currently_valid_members: u64,
    pub invalidated_after_dataset_creation: u64,
    pub integrity_failures: u64,
    pub missing_human_outcomes: u64,
    pub missing_requested_scorer_results: u64,
    pub included_members: u64,
    pub excluded_members: u64,
    pub no_change_count: u64,
    pub stage_change_count: u64,
    pub no_change_rate: Option<f64>,
    pub stage_change_rate: Option<f64>,
    pub destination_stage_counts: HashMap<String, u64>,
    pub transition_counts: HashMap<String, u64>,
    pub duplicate_content_groups: Vec<Vec<EvaluationCaseId>>,
    pub repeated_evidence_groups: Vec<Vec<EvaluationCaseId>>,
    pub repeated_source_meeting_groups: Vec<Vec<EvaluationCaseId>>,
    pub repeated_target_revision_groups: Vec<Vec<EvaluationCaseId>>,
    pub eligible_candidate_coverage: Option<f64>,
    pub missing_candidate_cases: Vec<EvaluationCaseId>,
    pub abstention_count: u64,
    pub invalid_candidate_count: u64,
    pub retry_count: u64,
    pub confidence_present_count: u64,
    pub confidence_missing_count: u64,
    pub warnings: Vec<EvaluationQualityWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationBinV1 {
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub upper_inclusive: bool,
    pub count: u64,
    pub mean_confidence: Option<f64>,
    pub empirical_agreement_rate: Option<f64>,
    pub absolute_gap: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceCalibrationV1 {
    pub revision: u16,
    pub calibrated_sample_count: u64,
    pub brier_score: Option<f64>,
    pub expected_calibration_error: Option<f64>,
    pub mean_confidence: Option<f64>,
    pub empirical_agreement_rate: Option<f64>,
    pub missing_confidence: u64,
    pub abstentions_excluded: u64,
    pub invalid_excluded: u64,
    pub retries_excluded: u64,
    pub invalidated_cases_excluded: u64,
    pub missing_outcomes_excluded: u64,
    pub unscored_candidates_excluded: u64,
    pub integrity_failures_excluded: u64,
    pub bins: Vec<CalibrationBinV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedHumanOutcomeEvidenceV1 {
    pub protocol_version: u16,
    pub case_id: EvaluationCaseId,
    pub outcome: HumanEvaluationOutcomeInput,
    pub normalized_stage: Value,
    pub operation_evidence: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationAnalysisCaseEvidenceV1 {
    pub case_id: EvaluationCaseId,
    pub case_bundle_digest: String,
    pub lifecycle_status: EvaluationCaseStatus,
    pub invalidation_evidence: Option<Value>,
    pub eligible_candidate: Option<EvaluationCandidate>,
    pub retry_candidates: Vec<EvaluationCandidate>,
    pub human_outcome: Option<VerifiedHumanOutcomeEvidenceV1>,
    pub requested_scorer_revision: u16,
    pub scorer_result: Option<EvaluationResult>,
    pub scorer_result_verified: bool,
    pub inclusion: String,
    pub content_identity_digest: String,
    pub meeting_evidence_digest: String,
    pub source_meeting_id: Option<String>,
    pub target_revision_identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationAnalysisEvidenceV1 {
    pub protocol_version: u16,
    pub dataset_id: EvaluationDatasetId,
    pub dataset_digest: String,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub target_actor_id: ActorId,
    pub target_actor_version: String,
    pub cases: Vec<EvaluationAnalysisCaseEvidenceV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationAnalysisReportV1 {
    pub protocol_version: u16,
    pub dataset_id: EvaluationDatasetId,
    pub dataset_digest: String,
    pub actor_id: ActorId,
    pub actor_version: String,
    pub scorer_revision: u16,
    pub readiness_rubric_revision: u16,
    pub quality_revision: u16,
    pub calibration_revision: u16,
    pub included_case_ids: Vec<EvaluationCaseId>,
    pub excluded_cases: HashMap<EvaluationCaseId, String>,
    pub metrics: EvaluationMetrics,
    pub quality: EvaluationQualityReportV1,
    pub calibration: ConfidenceCalibrationV1,
    pub advisory_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationAnalysisRun {
    pub id: EvaluationAnalysisRunId,
    pub analysis_input_digest: String,
    pub report_digest: String,
    pub created_at: DateTime<Utc>,
    pub evidence: EvaluationAnalysisEvidenceV1,
    pub report: EvaluationAnalysisReportV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationExportProfile {
    Metadata,
    FullLocal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationDatasetExport {
    pub payload: EvaluationDatasetExportPayload,
    pub export_digest: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationDatasetExportPayload {
    pub protocol_version: u16,
    pub profile: EvaluationExportProfile,
    pub sensitivity: String,
    pub contains_sensitive_values: bool,
    pub dataset: Value,
    pub analysis_runs: Value,
    pub cases: Value,
}

pub fn permitted_output_schema_v1() -> Value {
    serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema","protocol_version":1,"decision_types":["propose_stage_update","no_change","abstain"],"proposal":{"required":["object_id","record_id","field_id","expected_revision","value"],"additional_properties":false}})
}
pub fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), canonicalize_json(&map[key]));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        other => other.clone(),
    }
}
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, RowvaError> {
    let value = serde_json::to_value(value).map_err(|_| RowvaError::Internal {
        code: "canonical_serialization_failed".into(),
    })?;
    serde_json::to_vec(&canonicalize_json(&value)).map_err(|_| RowvaError::Internal {
        code: "canonical_serialization_failed".into(),
    })
}
pub fn sha256_digest<T: Serialize>(value: &T) -> Result<String, RowvaError> {
    Ok(Sha256::digest(canonical_json_bytes(value)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
pub fn evaluation_bundle_digest(bundle: &EvaluationCaseBundleV1) -> Result<String, RowvaError> {
    sha256_digest(bundle)
}
pub fn evaluation_case_content_digest(
    bundle: &EvaluationCaseBundleV1,
) -> Result<String, RowvaError> {
    sha256_digest(&serde_json::json!({
        "content_identity_revision": 1,
        "workflow": bundle.workflow,
        "workflow_version": bundle.workflow_version,
        "target_stage_field": bundle.target_stage_field,
        "base_schema_revision": bundle.base_schema_revision,
        "base_record_revision": bundle.base_record_revision,
        "record_snapshot": bundle.record_snapshot,
        "current_stage": bundle.current_stage,
        "input": bundle.input,
        "permitted_output_schema": bundle.permitted_output_schema,
    }))
}
pub fn evaluation_dataset_digest(
    manifest: &EvaluationDatasetManifestV1,
) -> Result<String, RowvaError> {
    sha256_digest(manifest)
}
pub fn candidate_fingerprint(input: &CandidateImport) -> Result<String, RowvaError> {
    sha256_digest(
        &serde_json::json!({"protocol_version":input.protocol_version,"case_id":input.case_id,"bundle_digest":input.bundle_digest,"actor_id":input.actor.id,"actor_type":input.actor.actor_type,"actor_version":input.actor.actor_version,"client_name":input.actor.client_name,"session_id":input.actor.session_id,"decision":input.decision,"reason":input.reason,"confidence":input.confidence}),
    )
}

pub fn validate_evaluation_actor(
    actor: &ActorContext,
    candidate: bool,
) -> Result<ActorContext, RowvaError> {
    check_len(
        "actor_display_name",
        &actor.display_name,
        MAX_ACTOR_DISPLAY_NAME_BYTES,
    )?;
    if let Some(v) = &actor.actor_version {
        check_len("actor_version", v, MAX_ACTOR_VERSION_BYTES)?;
    }
    if let Some(v) = &actor.client_name {
        check_len("client_name", v, MAX_CLIENT_NAME_BYTES)?;
    }
    if let Some(v) = &actor.session_id {
        check_len("session_id", v, MAX_SESSION_ID_BYTES)?;
    }
    if candidate
        && (actor.actor_type != ActorType::Agent && actor.actor_type != ActorType::Integration)
    {
        return Err(RowvaError::validation(
            "invalid_candidate_actor",
            "candidate actor must be agent or integration",
        ));
    }
    if candidate && actor.actor_version.as_deref().is_none_or(str::is_empty) {
        return Err(RowvaError::validation(
            "actor_version_required",
            "candidate actor_version is required",
        ));
    }
    let mut sanitized = actor.clone();
    sanitized.capabilities.clear();
    Ok(sanitized)
}
pub fn validate_evaluation_texts(input: &DealStageQualificationInputV1) -> Result<(), RowvaError> {
    if let Some(v) = &input.source_meeting_id {
        check_len("source_meeting_id", v, MAX_SOURCE_MEETING_ID_BYTES)?;
    }
    if serde_json::to_vec(&input.meeting_evidence)
        .map_err(|_| {
            RowvaError::validation("invalid_evidence", "meeting evidence cannot be serialized")
        })?
        .len()
        > MAX_MEETING_EVIDENCE_BYTES
    {
        return Err(RowvaError::validation(
            "evaluation_input_too_large",
            "meeting evidence exceeds 65536 bytes",
        ));
    }
    Ok(())
}
pub fn validate_candidate_texts(input: &CandidateImport) -> Result<(), RowvaError> {
    if input
        .confidence
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err(RowvaError::validation(
            "invalid_confidence",
            "confidence must be finite and between 0 and 1",
        ));
    }
    if let Some(v) = &input.reason {
        check_len("candidate_reason", v, MAX_REASON_BYTES)?;
    }
    match &input.decision {
        ShadowDecision::NoChange { reason: Some(v) } => {
            check_len("no_change_reason", v, MAX_REASON_BYTES)?
        }
        ShadowDecision::Abstain { reason } => {
            check_len("abstain_reason", reason, MAX_REASON_BYTES)?;
            if reason.is_empty() {
                return Err(RowvaError::validation(
                    "abstain_reason_required",
                    "abstain reason is required",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_scorer_revision(revision: u16) -> Result<(), RowvaError> {
    if revision == EVALUATION_SCORER_REVISION {
        Ok(())
    } else {
        Err(RowvaError::validation(
            "unsupported_evaluation_scorer_revision",
            "unsupported evaluation scorer revision",
        ))
    }
}
pub fn validate_readiness_revision(revision: u16) -> Result<(), RowvaError> {
    if revision == EVALUATION_READINESS_REVISION {
        Ok(())
    } else {
        Err(RowvaError::validation(
            "unsupported_evaluation_readiness_revision",
            "unsupported evaluation readiness rubric revision",
        ))
    }
}

pub fn confidence_calibration(samples: &[(f64, bool)]) -> ConfidenceCalibrationV1 {
    let mut bins = (0..10)
        .map(|index| CalibrationBinV1 {
            lower_bound: index as f64 / 10.0,
            upper_bound: (index + 1) as f64 / 10.0,
            upper_inclusive: index == 9,
            count: 0,
            mean_confidence: None,
            empirical_agreement_rate: None,
            absolute_gap: None,
        })
        .collect::<Vec<_>>();
    let mut sums = [(0.0, 0u64); 10];
    let mut brier = 0.0;
    let mut confidence_sum = 0.0;
    let mut correct = 0u64;
    for (confidence, agreement) in samples {
        let index = if *confidence == 1.0 {
            9
        } else {
            (*confidence * 10.0).floor() as usize
        };
        bins[index].count += 1;
        sums[index].0 += confidence;
        sums[index].1 += u64::from(*agreement);
        let target = if *agreement { 1.0 } else { 0.0 };
        brier += (confidence - target).powi(2);
        confidence_sum += confidence;
        correct += u64::from(*agreement);
    }
    let count = samples.len() as u64;
    let mut ece = 0.0;
    for (index, bin) in bins.iter_mut().enumerate() {
        if bin.count > 0 {
            let mean = sums[index].0 / bin.count as f64;
            let rate = sums[index].1 as f64 / bin.count as f64;
            let gap = (mean - rate).abs();
            bin.mean_confidence = Some(mean);
            bin.empirical_agreement_rate = Some(rate);
            bin.absolute_gap = Some(gap);
            ece += gap * bin.count as f64 / count as f64;
        }
    }
    ConfidenceCalibrationV1 {
        revision: EVALUATION_CALIBRATION_REVISION,
        calibrated_sample_count: count,
        brier_score: (count > 0).then_some(brier / count as f64),
        expected_calibration_error: (count > 0).then_some(ece),
        mean_confidence: (count > 0).then_some(confidence_sum / count as f64),
        empirical_agreement_rate: (count > 0).then_some(correct as f64 / count as f64),
        missing_confidence: 0,
        abstentions_excluded: 0,
        invalid_excluded: 0,
        retries_excluded: 0,
        invalidated_cases_excluded: 0,
        missing_outcomes_excluded: 0,
        unscored_candidates_excluded: 0,
        integrity_failures_excluded: 0,
        bins,
    }
}
fn check_len(name: &str, value: &str, max: usize) -> Result<(), RowvaError> {
    if value.len() > max {
        Err(RowvaError::validation(
            format!("{name}_too_long"),
            format!("{name} exceeds {max} bytes"),
        ))
    } else {
        Ok(())
    }
}

pub fn plan_frozen_candidate(
    case: &EvaluationCase,
    decision: &ShadowDecision,
) -> Result<Vec<ProposedChange>, (CandidateValidationStatus, RowvaError)> {
    let bundle = &case.bundle;
    match decision {
        ShadowDecision::NoChange { .. } | ShadowDecision::Abstain { .. } => Ok(vec![]),
        ShadowDecision::ProposeStageUpdate { proposal } => {
            if proposal.object_id != bundle.target_object_id
                || proposal.record_id != bundle.target_record_id
            {
                return Err((
                    CandidateValidationStatus::InvalidTarget,
                    RowvaError::validation(
                        "evaluation_invalid_target",
                        "proposal target does not match the frozen case",
                    ),
                ));
            }
            if proposal.field_id != bundle.target_stage_field.id {
                return Err((
                    CandidateValidationStatus::UnsafeExtraChanges,
                    RowvaError::validation(
                        "evaluation_unsafe_field",
                        "proposal may change only the frozen stage field",
                    ),
                ));
            }
            if proposal.expected_revision != bundle.base_record_revision {
                return Err((
                    CandidateValidationStatus::StaleRevision,
                    RowvaError::validation(
                        "evaluation_stale_revision",
                        "proposal revision does not match the frozen case",
                    ),
                ));
            }
            if !field_value_is_valid(&bundle.target_stage_field, &proposal.value) {
                return Err((
                    CandidateValidationStatus::InvalidValue,
                    RowvaError::validation(
                        "evaluation_invalid_value",
                        "stage value is incompatible with the frozen field",
                    ),
                ));
            }
            Ok(vec![ProposedChange {
                object_id: proposal.object_id.clone(),
                record_id: Some(proposal.record_id.clone()),
                field_id: Some(proposal.field_id.clone()),
                before: Some(bundle.current_stage.clone()),
                after: Some(proposal.value.clone()),
            }])
        }
    }
}
pub fn field_value_is_valid(field: &FieldDefinition, value: &Value) -> bool {
    field_kind_value_is_valid(&field.kind, field.required, value)
}
pub fn field_kind_value_is_valid(kind: &FieldKind, required: bool, value: &Value) -> bool {
    if value.is_null() {
        return !required;
    }
    match kind {
        FieldKind::Text(_) | FieldKind::Date | FieldKind::DateTime => value.is_string(),
        FieldKind::Number(_) => value.is_number(),
        FieldKind::Boolean => value.is_boolean(),
        FieldKind::Enum(c) => value
            .as_str()
            .is_some_and(|v| c.options.iter().any(|o| o == v)),
        _ => false,
    }
}

pub fn score_candidate(
    candidate: &EvaluationCandidate,
    human_stage: &Value,
    current_stage: &Value,
    now: DateTime<Utc>,
) -> EvaluationResult {
    let candidate_stage = candidate
        .normalized_changes
        .first()
        .and_then(|c| c.after.clone());
    let verdict = match candidate.validation_status {
        CandidateValidationStatus::InvalidTarget => EvaluationVerdict::InvalidTarget,
        CandidateValidationStatus::StaleRevision => EvaluationVerdict::StaleRevision,
        CandidateValidationStatus::InvalidValue => EvaluationVerdict::InvalidValue,
        CandidateValidationStatus::UnsafeExtraChanges => EvaluationVerdict::UnsafeExtraChanges,
        CandidateValidationStatus::Invalid => EvaluationVerdict::InvalidCandidate,
        CandidateValidationStatus::Valid => match &candidate.decision {
            ShadowDecision::Abstain { .. } => EvaluationVerdict::Abstained,
            ShadowDecision::NoChange { .. } if human_stage == current_stage => {
                EvaluationVerdict::NoChangeAgreement
            }
            ShadowDecision::NoChange { .. } => EvaluationVerdict::FalseNegative,
            ShadowDecision::ProposeStageUpdate { .. } if human_stage == current_stage => {
                EvaluationVerdict::FalsePositive
            }
            ShadowDecision::ProposeStageUpdate { .. }
                if candidate_stage.as_ref() == Some(human_stage) =>
            {
                EvaluationVerdict::ExactAgreement
            }
            _ => EvaluationVerdict::WrongStage,
        },
    };
    EvaluationResult {
        id: EvaluationResultId::new(),
        case_id: candidate.case_id.clone(),
        candidate_id: candidate.id.clone(),
        scorer_revision: 1,
        candidate_stage,
        human_stage: human_stage.clone(),
        verdict,
        eligible: candidate.eligible_for_metrics,
        scored_at: now,
    }
}

pub fn readiness(counts: &EvaluationCounts) -> (ReadinessClass, Vec<String>) {
    if counts.scored_eligible < 20 {
        return (
            ReadinessClass::InsufficientEvidence,
            vec!["at least 20 eligible scored decisions are required".into()],
        );
    }
    let accuracy_denominator = counts.scored_eligible.saturating_sub(counts.abstentions);
    if accuracy_denominator == 0 {
        return (
            ReadinessClass::InsufficientEvidence,
            vec!["no non-abstaining decisions are available".into()],
        );
    }
    let coverage = accuracy_denominator as f64 / counts.scored_eligible as f64;
    if coverage < 0.8 {
        return (
            ReadinessClass::NotReady,
            vec!["coverage is below 80%; abstentions reduce coverage".into()],
        );
    }
    let errors = counts.false_positives
        + counts.false_negatives
        + counts.wrong_stage
        + counts.invalid_scored;
    if errors * 10 > accuracy_denominator {
        return (
            ReadinessClass::NotReady,
            vec!["error and invalid rate exceeds 10% of non-abstaining decisions".into()],
        );
    }
    if counts.agreement_count * 100 >= accuracy_denominator * 90 {
        return (
            ReadinessClass::CandidateForHumanReview,
            vec!["agreement is at least 90% with sufficient evidence and coverage".into()],
        );
    }
    (
        ReadinessClass::Promising,
        vec!["sufficient evidence but the 90% agreement threshold is not met".into()],
    )
}

pub trait EvaluationApplication {
    fn create_evaluation_case(
        &mut self,
        input: EvaluationCaseCreate,
    ) -> Result<EvaluationCase, RowvaError>;
    fn get_evaluation_case(&self, id: &EvaluationCaseId) -> Result<EvaluationCase, RowvaError>;
    fn export_evaluation_case(
        &self,
        id: &EvaluationCaseId,
    ) -> Result<EvaluationCaseExport, RowvaError>;
    fn submit_evaluation_candidate(
        &mut self,
        input: CandidateImport,
    ) -> Result<EvaluationCandidate, RowvaError>;
    fn record_evaluation_outcome(
        &mut self,
        id: &EvaluationCaseId,
        input: HumanEvaluationOutcomeInput,
    ) -> Result<Vec<EvaluationResult>, RowvaError>;
    fn replay_evaluation_candidate(
        &self,
        case_id: &EvaluationCaseId,
        candidate_id: &EvaluationCandidateId,
    ) -> Result<FrozenReplayResult, RowvaError>;
    fn evaluation_report(&self) -> Result<Vec<EvaluationMetrics>, RowvaError>;
    fn invalidate_evaluation_case(
        &mut self,
        input: EvaluationInvalidationInput,
    ) -> Result<EvaluationInvalidation, RowvaError>;
    fn list_evaluation_cases(&self) -> Result<Vec<EvaluationCase>, RowvaError>;
    fn create_evaluation_dataset(
        &mut self,
        input: EvaluationDatasetCreateV1,
    ) -> Result<EvaluationDataset, RowvaError>;
    fn get_evaluation_dataset(
        &self,
        id: &EvaluationDatasetId,
    ) -> Result<EvaluationDataset, RowvaError>;
    fn list_evaluation_datasets(&self) -> Result<Vec<EvaluationDataset>, RowvaError>;
    fn run_evaluation_analysis(
        &mut self,
        input: EvaluationAnalysisRequestV1,
    ) -> Result<EvaluationAnalysisRun, RowvaError>;
    fn get_evaluation_analysis(
        &self,
        id: &EvaluationAnalysisRunId,
    ) -> Result<EvaluationAnalysisRun, RowvaError>;
    fn list_evaluation_analyses(&self) -> Result<Vec<EvaluationAnalysisRun>, RowvaError>;
    fn export_evaluation_dataset(
        &self,
        id: &EvaluationDatasetId,
        profile: EvaluationExportProfile,
    ) -> Result<EvaluationDatasetExport, RowvaError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn counts(change: u64, no_change: u64, errors: u64, abstain: u64) -> EvaluationCounts {
        EvaluationCounts {
            distinct_cases_attempted: change + no_change + errors + abstain,
            eligible_decisions: change + no_change + errors + abstain,
            scored_eligible: change + no_change + errors + abstain,
            exact_change_agreements: change,
            no_change_agreements: no_change,
            agreement_count: change + no_change,
            false_positives: errors,
            abstentions: abstain,
            ..Default::default()
        }
    }
    #[test]
    fn ids_and_workflow_are_stable() {
        let id = EvaluationCaseId::new();
        assert_eq!(EvaluationCaseId::from_string(id.to_string()).unwrap(), id);
        assert_eq!(
            serde_json::to_string(&EvaluationWorkflow::DealStageQualificationV1).unwrap(),
            "\"deal_stage_qualification_v1\""
        );
    }
    #[test]
    fn readiness_counts_both_agreement_types() {
        assert_eq!(
            readiness(&counts(20, 0, 0, 0)).0,
            ReadinessClass::CandidateForHumanReview
        );
        assert_eq!(
            readiness(&counts(0, 20, 0, 0)).0,
            ReadinessClass::CandidateForHumanReview
        );
        assert_eq!(
            readiness(&counts(10, 10, 0, 0)).0,
            ReadinessClass::CandidateForHumanReview
        );
    }
    #[test]
    fn readiness_boundaries_and_empty_denominators() {
        assert_eq!(
            readiness(&counts(18, 0, 2, 0)).0,
            ReadinessClass::CandidateForHumanReview
        );
        assert_eq!(readiness(&counts(17, 0, 3, 0)).0, ReadinessClass::NotReady);
        assert_eq!(readiness(&counts(19, 0, 0, 6)).0, ReadinessClass::NotReady);
        assert_eq!(
            readiness(&EvaluationCounts::default()).0,
            ReadinessClass::InsufficientEvidence
        );
        let mut ignored = counts(20, 0, 0, 0);
        ignored.ineligible_retries = 100;
        ignored.pending_unscored_eligible = 100;
        assert_eq!(
            readiness(&ignored).0,
            ReadinessClass::CandidateForHumanReview
        );
    }
    #[test]
    fn strict_candidate_rejects_general_or_unknown_payloads() {
        let raw = serde_json::json!({"protocol_version":1,"case_id":EvaluationCaseId::new(),"bundle_digest":"x","actor":ActorContext::local_user(),"decision":{"type":"propose_stage_update","proposal":{"object_id":ObjectId::new(),"record_id":RecordId::new(),"field_id":FieldId::new(),"expected_revision":1,"value":"Qualified","extra":true}},"reason":null,"confidence":null});
        assert!(serde_json::from_value::<CandidateImport>(raw).is_err());
    }

    #[test]
    fn confidence_calibration_has_stable_bins_and_scores() {
        let report = confidence_calibration(&[(1.0, true), (0.0, false), (0.9, false)]);
        assert_eq!(report.calibrated_sample_count, 3);
        assert_eq!(report.bins[0].count, 1);
        assert_eq!(report.bins[9].count, 2);
        assert!((report.brier_score.unwrap() - 0.27).abs() < 1e-12);
        assert!(report.expected_calibration_error.is_some());
        let empty = confidence_calibration(&[]);
        assert_eq!(empty.brier_score, None);
        assert_eq!(empty.expected_calibration_error, None);
    }

    #[test]
    fn revision_dispatch_and_new_ids_are_strict() {
        assert!(EvaluationDatasetId::from_string(EvaluationDatasetId::new().to_string()).is_ok());
        assert!(EvaluationInvalidationId::from_string("wrong").is_err());
        assert!(validate_scorer_revision(1).is_ok());
        assert!(
            matches!(validate_scorer_revision(2),Err(RowvaError::Validation{code,..}) if code=="unsupported_evaluation_scorer_revision")
        );
        assert!(
            matches!(validate_readiness_revision(2),Err(RowvaError::Validation{code,..}) if code=="unsupported_evaluation_readiness_revision")
        );
    }

    #[test]
    fn confidence_validation_rejects_non_finite_and_out_of_range() {
        let mut input = CandidateImport {
            protocol_version: 1,
            case_id: EvaluationCaseId::new(),
            bundle_digest: "digest".into(),
            actor: ActorContext::local_user(),
            decision: ShadowDecision::NoChange { reason: None },
            reason: None,
            confidence: Some(f64::NAN),
        };
        assert!(validate_candidate_texts(&input).is_err());
        input.confidence = Some(1.01);
        assert!(validate_candidate_texts(&input).is_err());
        input.confidence = Some(0.0);
        assert!(validate_candidate_texts(&input).is_ok());
        input.confidence = Some(1.0);
        assert!(validate_candidate_texts(&input).is_ok());
    }
}
