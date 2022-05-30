use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Datapoint {
    GameStart(GameStartDatapoint),
    GameInspection(GameInspectionDatapoint),
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
pub struct GameInspectionDatapoint {
    // Unique identifier of this game
    pub game_id: String,
    // How long the inspection period lasted, in milliseconds
    pub inspection_milliseconds_duration: u32,
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
    // How long since the inspection period ended, in milliseconds
    pub play_milliseconds_elapsed: Option<u32>,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSolveDatapoint {
    // Unique identifier of this game
    pub game_id: String,
    // How long the inspection period lasted, in milliseconds
    pub inspection_milliseconds_duration: u32,
    // How long the play period lasted, in milliseconds
    pub play_milliseconds_duration: u32,
    // How many twists were performed during the game
    pub number_of_twists: u32,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}
