CREATE TABLE battles (
  id UUID PRIMARY KEY,
  user1_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  user2_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  level INTEGER NOT NULL,
  task_text TEXT,
  start_at TIMESTAMPTZ NOT NULL,
  end_at TIMESTAMPTZ,
  score_user1 INTEGER NOT NULL DEFAULT 0,
  score_user2 INTEGER NOT NULL DEFAULT 0,
  elo_delta_user1 INTEGER NOT NULL DEFAULT 0,
  elo_delta_user2 INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX battles_user1_idx ON battles (user1_id);
CREATE INDEX battles_user2_idx ON battles (user2_id);
