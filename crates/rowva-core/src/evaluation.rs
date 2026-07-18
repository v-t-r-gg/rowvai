use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt};
use uuid::Uuid;

macro_rules! evaluation_id {
    ($name:ident, $prefix:literal) => {
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
pub struct EvaluationCase {
    pub id: EvaluationCaseId,
    pub workspace_id: WorkspaceId,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub target_object_id: ObjectId,
    pub target_record_id: RecordId,
    pub target_stage_field: FieldDefinition,
    pub base_schema_revision: SchemaRevision,
    pub base_record_revision: RecordRevision,
    pub current_stage: Value,
    pub record_snapshot: HashMap<FieldId, Value>,
    pub input: DealStageQualificationInputV1,
    pub evidence_digest: String,
    pub bundle_digest: String,
    pub creator: ActorContext,
    pub created_at: DateTime<Utc>,
    pub status: EvaluationCaseStatus,
    pub invalidated_at: Option<DateTime<Utc>>,
    pub invalidation_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShadowDecision {
    ProposeStageUpdate { command: Command },
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
    pub decision: ShadowDecision,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub fingerprint: String,
    pub submitted_at: DateTime<Utc>,
    pub validation_status: CandidateValidationStatus,
    pub validation_error: Option<RowvaError>,
    pub eligible_for_metrics: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationCaseExport {
    pub protocol_version: u16,
    pub case_id: EvaluationCaseId,
    pub workflow: EvaluationWorkflow,
    pub workflow_version: u16,
    pub bundle_digest: String,
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
    pub total_cases: u64,
    pub eligible_cases: u64,
    pub candidates_submitted: u64,
    pub valid_candidates: u64,
    pub invalid_candidates: u64,
    pub scored_candidates: u64,
    pub exact_agreements: u64,
    pub no_change_agreements: u64,
    pub false_positives: u64,
    pub false_negatives: u64,
    pub wrong_stage: u64,
    pub abstentions: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub workflow: EvaluationWorkflow,
    pub actor_id: ActorId,
    pub actor_version: String,
    pub counts: EvaluationCounts,
    pub exact_agreement_rate: f64,
    pub coverage_rate: f64,
    pub invalid_proposal_rate: f64,
    pub readiness_rubric_revision: u16,
    pub readiness: ReadinessClass,
    pub readiness_reasons: Vec<String>,
    pub advisory_only: bool,
}

pub fn plan_frozen_candidate(
    case: &EvaluationCase,
    decision: &ShadowDecision,
) -> Result<Vec<ProposedChange>, (CandidateValidationStatus, RowvaError)> {
    match decision {
        ShadowDecision::NoChange { .. } | ShadowDecision::Abstain { .. } => Ok(vec![]),
        ShadowDecision::ProposeStageUpdate { command } => match command {
            Command::UpdateRecord {
                object_id,
                record_id,
                values,
                expected_revision,
            } => {
                if object_id != &case.target_object_id || record_id != &case.target_record_id {
                    return Err((
                        CandidateValidationStatus::InvalidTarget,
                        RowvaError::validation(
                            "evaluation_invalid_target",
                            "candidate targets a different object or record",
                        ),
                    ));
                }
                if *expected_revision != Some(case.base_record_revision) {
                    return Err((
                        CandidateValidationStatus::StaleRevision,
                        RowvaError::validation(
                            "evaluation_stale_revision",
                            "candidate revision does not match the frozen case",
                        ),
                    ));
                }
                if values.len() != 1 || !values.contains_key(&case.target_stage_field.id) {
                    return Err((
                        CandidateValidationStatus::UnsafeExtraChanges,
                        RowvaError::validation(
                            "evaluation_unsafe_extra_changes",
                            "candidate may update only the frozen stage field",
                        ),
                    ));
                }
                let after = values[&case.target_stage_field.id].clone();
                if !field_value_is_valid(&case.target_stage_field, &after) {
                    return Err((
                        CandidateValidationStatus::InvalidValue,
                        RowvaError::validation(
                            "evaluation_invalid_value",
                            "stage value is incompatible with the frozen field definition",
                        ),
                    ));
                }
                Ok(vec![ProposedChange {
                    object_id: object_id.clone(),
                    record_id: Some(record_id.clone()),
                    field_id: Some(case.target_stage_field.id.clone()),
                    before: Some(case.current_stage.clone()),
                    after: Some(after),
                }])
            }
            _ => Err((
                CandidateValidationStatus::Invalid,
                RowvaError::validation(
                    "evaluation_invalid_command",
                    "only update_record is accepted",
                ),
            )),
        },
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
    if counts.scored_candidates < 20 {
        return (
            ReadinessClass::InsufficientEvidence,
            vec!["at least 20 scored candidates are required".into()],
        );
    }
    let errors = counts.false_positives
        + counts.false_negatives
        + counts.wrong_stage
        + counts.invalid_candidates;
    if errors * 10 > counts.scored_candidates {
        return (
            ReadinessClass::NotReady,
            vec!["error or invalid rate exceeds 10%".into()],
        );
    }
    if counts.exact_agreements * 100 >= counts.scored_candidates * 90 {
        return (
            ReadinessClass::CandidateForHumanReview,
            vec!["at least 90% exact agreement with sufficient evidence".into()],
        );
    }
    (
        ReadinessClass::Promising,
        vec!["sufficient evidence but conservative review threshold not met".into()],
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn readiness_is_conservative() {
        assert_eq!(
            readiness(&EvaluationCounts::default()).0,
            ReadinessClass::InsufficientEvidence
        );
        let c = EvaluationCounts {
            scored_candidates: 20,
            exact_agreements: 18,
            ..Default::default()
        };
        assert_eq!(readiness(&c).0, ReadinessClass::CandidateForHumanReview);
    }
}
