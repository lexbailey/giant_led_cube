use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Datapoint {
    GameStart(GameStartDatapoint),
    Twist(TwistDatapoint),
    GameSolve(GameSolveDatapoint),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartDatapoint {
    // Unique identifier of this game
    pub game_id: String,
    // Starting cube face positions
    pub cube_state: String,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwistDatapoint {
    // The cube rotation in standard notation https://ruwix.com/the-rubiks-cube/notation/
    pub rotation: String,
    // New cube face positions
    pub updated_cube_state: String,
    // Game unique ID (only if this twist happened during an active game)
    pub game_id: Option<String>,
    // How long the solve attempt has taken so far, in milliseconds
    pub play_time_milliseconds: Option<u32>,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSolveDatapoint {
    // Unique identifier of this game
    pub game_id: String,
    // How long the solve took, in milliseconds
    pub play_time_milliseconds: u32,
    // Whether the cube recognised this as a new top score
    pub new_top_score: bool,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}
