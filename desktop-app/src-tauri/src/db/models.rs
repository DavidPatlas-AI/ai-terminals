// The Box — Phase 3 row structs. Field order/names mirror schema.sql /
// THE_BOX_AUDIT/05_DATABASE_AND_CONVERSATION_MODEL_HE.md exactly.

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    pub created_at: String,
    pub last_opened_at: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub id: String,
    pub conversation_id: String,
    pub seq: i64,
    pub user_intent: String,
    pub task_plan_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRun {
    pub id: String,
    pub turn_id: String,
    pub provider: String,
    pub provider_session_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub exit_status: Option<String>,
    pub raw_log_ref: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskPlan {
    pub id: String,
    pub turn_id: String,
    pub steps_json: String,
    pub risk_level: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub turn_id: String,
    pub scope: String,
    pub action_ref: Option<String>,
    pub decision: String,
    pub decided_at: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub id: String,
    pub turn_id: String,
    pub provider_run_id: Option<String>,
    pub file_path: String,
    pub change_type: String,
    pub diff_ref: Option<String>,
    pub git_checkpoint_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_decision: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub turn_id: String,
    pub kind: String,
    pub content_ref: String,
    pub created_at: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitCheckpoint {
    pub id: String,
    pub turn_id: String,
    pub project_id: String,
    pub commit_or_stash_ref: String,
    pub created_at: String,
    pub restored_at: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenQuestion {
    pub id: String,
    pub turn_id: String,
    pub question: String,
    pub raised_by_provider_run_id: Option<String>,
    pub resolved_at: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub turn_id: String,
    pub provider_run_id: Option<String>,
    pub level: String,
    pub message: String,
    pub created_at: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}
