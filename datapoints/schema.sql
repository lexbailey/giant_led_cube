-- ClickHouse schema for the Giant LED Cube.
-- More information on ClickHouse schemas:
--   https://clickhouse.com/docs/en/engines/table-engines/mergetree-family/mergetree
--   https://clickhouse.com/docs/en/sql-reference

CREATE TABLE game_starts (
    game_id String COMMENT 'Unique identifier of this game',
    cube_state String COMMENT 'Starting cube face positions',
    timestamp DateTime64(6, 'UTC') COMMENT 'Milliseconds since the unix epoch'
) ENGINE MergeTree() 
  PARTITION BY toYYYYMM(timestamp) 
  ORDER BY (timestamp, game_id)
  SETTINGS index_granularity=8192
  COMMENT 'Records new games starting with a randomised cube';

CREATE TABLE twists (
    rotation String COMMENT 'The cube rotation in standard notation https://ruwix.com/the-rubiks-cube/notation/',
    cube_state String COMMENT 'Updated cube face positions',
    game_id Nullable(String) COMMENT 'Game unique ID (only if this twist happened during an active game)',
    play_time_milliseconds Nullable(UInt32) COMMENT 'How long the solve attempt has taken so far, in milliseconds',
    timestamp DateTime64(6, 'UTC') COMMENT 'Milliseconds since the unix epoch'
) ENGINE MergeTree() 
  PARTITION BY toYYYYMM(timestamp) 
  ORDER BY (timestamp)
  SETTINGS index_granularity=8192
  COMMENT 'Records cube twists (rotations) and whether they are part of a game';

CREATE TABLE game_solves (
    game_id String COMMENT 'Unique identifier of this game',
    play_time_milliseconds UInt32 COMMENT 'How long the solve took, in milliseconds',
    new_top_score Bool COMMENT 'Whether the cube recognised this as a new top score',
    cube_state String COMMENT 'Solved cube face positions',
    timestamp DateTime64(6, 'UTC') COMMENT 'Milliseconds since the unix epoch'
) ENGINE MergeTree() 
  PARTITION BY toYYYYMM(timestamp) 
  ORDER BY (timestamp, game_id)
  SETTINGS index_granularity=8192
  COMMENT 'Records successful solves starting from a randomised cube';
