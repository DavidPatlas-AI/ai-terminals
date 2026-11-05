-- The Box — Phase 3 schema.
-- Source of truth: THE_BOX_AUDIT/05_DATABASE_AND_CONVERSATION_MODEL_HE.md
-- Applied at startup via db::init(); every statement is idempotent
-- (CREATE TABLE IF NOT EXISTS) so re-running on an existing DB is a no-op.

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  icon TEXT,
  created_at TEXT NOT NULL,
  last_opened_at TEXT
);

CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  title TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('active','completed','archived'))
);

CREATE TABLE IF NOT EXISTS task_plans (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL,
  steps_json TEXT NOT NULL,
  risk_level TEXT CHECK(risk_level IN ('low','medium','high')),
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS turns (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id),
  seq INTEGER NOT NULL,
  user_intent TEXT NOT NULL,
  task_plan_id TEXT REFERENCES task_plans(id),
  status TEXT NOT NULL CHECK(status IN ('planning','awaiting_approval','running','blocked','completed','failed','cancelled')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_runs (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  provider TEXT NOT NULL CHECK(provider IN ('claude_code','codex','gemini','ollama','grok','deepseek')),
  provider_session_id TEXT,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  exit_status TEXT CHECK(exit_status IN ('success','error','cancelled', NULL)),
  raw_log_ref TEXT
);

CREATE TABLE IF NOT EXISTS approvals (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  scope TEXT NOT NULL,
  action_ref TEXT,
  decision TEXT NOT NULL CHECK(decision IN ('approved','rejected')),
  decided_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS git_checkpoints (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  project_id TEXT NOT NULL REFERENCES projects(id),
  commit_or_stash_ref TEXT NOT NULL,
  created_at TEXT NOT NULL,
  restored_at TEXT
);

CREATE TABLE IF NOT EXISTS file_changes (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  provider_run_id TEXT REFERENCES provider_runs(id),
  file_path TEXT NOT NULL,
  change_type TEXT NOT NULL CHECK(change_type IN ('created','modified','deleted')),
  diff_ref TEXT,
  git_checkpoint_id TEXT REFERENCES git_checkpoints(id),
  reviewed_at TEXT,
  review_decision TEXT CHECK(review_decision IN ('kept','reverted', NULL))
);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  kind TEXT NOT NULL,
  content_ref TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS open_questions (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  question TEXT NOT NULL,
  raised_by_provider_run_id TEXT REFERENCES provider_runs(id),
  resolved_at TEXT,
  resolution TEXT
);

CREATE TABLE IF NOT EXISTS logs (
  id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL REFERENCES turns(id),
  provider_run_id TEXT REFERENCES provider_runs(id),
  level TEXT NOT NULL CHECK(level IN ('info','warn','error')),
  message TEXT NOT NULL,
  created_at TEXT NOT NULL
);

-- No secrets in this table — see THE_BOX_AUDIT doc 05 "הפרדת Secrets".
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
