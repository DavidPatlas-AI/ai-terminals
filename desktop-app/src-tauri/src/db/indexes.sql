CREATE INDEX IF NOT EXISTS turns_by_conversation_seq ON turns(conversation_id,seq);
CREATE INDEX IF NOT EXISTS logs_by_turn_created ON logs(turn_id,created_at);
CREATE INDEX IF NOT EXISTS runs_by_turn ON provider_runs(turn_id);
CREATE INDEX IF NOT EXISTS runs_by_provider_started ON provider_runs(provider,started_at);
CREATE INDEX IF NOT EXISTS conversations_by_project ON conversations(project_id,created_at);
