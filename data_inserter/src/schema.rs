use chrono::{DateTime, Utc};
use reqwest::StatusCode;
#[cfg(target_arch = "wasm32")]
use reqwest_wasm_ext::ReqwestExt;
use serde::{Deserialize, Serialize};
use std::result::Result;

use crate::utils::ClickhouseConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Datapoint {
    GameStart(GameStartDatapoint),
    Twist(TwistDatapoint),
    GameSolve(GameSolveDatapoint),
}

impl Datapoint {
    pub async fn insert_to_clickhouse(
        &self,
        clickhouse_config: &ClickhouseConfig,
    ) -> Result<(), String> {
        use Datapoint::*;
        match self {
            GameStart(v) => v.insert_to_clickhouse(clickhouse_config).await,
            Twist(v) => v.insert_to_clickhouse(clickhouse_config).await,
            GameSolve(v) => v.insert_to_clickhouse(clickhouse_config).await,
        }
    }
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

impl GameStartDatapoint {
    pub async fn insert_to_clickhouse(
        &self,
        clickhouse_config: &ClickhouseConfig,
    ) -> Result<(), String> {
        insert_to_clickhouse(self, "game_starts", clickhouse_config).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwistDatapoint {
    // The cube rotation in standard notation https://ruwix.com/the-rubiks-cube/notation/
    pub rotation: String,
    // New cube face positions
    pub cube_state: String,
    // Game unique ID (only if this twist happened during an active game)
    pub game_id: Option<String>,
    // How long the solve attempt has taken so far, in milliseconds
    pub play_time_milliseconds: Option<u32>,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}

impl TwistDatapoint {
    pub async fn insert_to_clickhouse(
        &self,
        clickhouse_config: &ClickhouseConfig,
    ) -> Result<(), String> {
        insert_to_clickhouse(self, "twists", clickhouse_config).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSolveDatapoint {
    // Unique identifier of this game
    pub game_id: String,
    // How long the solve took, in milliseconds
    pub play_time_milliseconds: u32,
    // Whether the cube recognised this as a new top score
    pub new_top_score: bool,
    // Solved cube face positions
    pub cube_state: String,
    // Time since the unix epoch
    pub timestamp: DateTime<Utc>,
}

impl GameSolveDatapoint {
    pub async fn insert_to_clickhouse(
        &self,
        clickhouse_config: &ClickhouseConfig,
    ) -> Result<(), String> {
        insert_to_clickhouse(self, "game_solves", clickhouse_config).await
    }
}

async fn insert_to_clickhouse(
    item: impl Serialize,
    table_name: &str,
    clickhouse_config: &ClickhouseConfig,
) -> Result<(), String> {
    let json =
        serde_json::to_string(&item).map_err(|e| format!("error serialising to JSON: {}", e))?;
    let sql = format!("INSERT INTO {:?} FORMAT JSONEachRow {}", table_name, json);
    let client = reqwest::Client::new();
    let req = client
        .post(&clickhouse_config.url)
        .body(sql.clone())
        .basic_auth(&clickhouse_config.user, Some(&clickhouse_config.password));
    // FIXME: Implement a WASM-compatible timeout
    //.timeout(Duration::from_secs(5));

    let resp = req
        .send()
        .await
        .map_err(|e| format!("error sending request to ClickHouse: {}", e))?;
    let status_code = resp.status();
    if status_code != StatusCode::OK {
        let error_body = resp.text().await;
        return Err(format!(
            "unexpected non-200 status code from ClickHouse: {}: response body {:?} from input: {}",
            status_code,
            error_body.unwrap_or_else(|_| "".to_string()),
            sql
        ));
    }
    Ok(())
}
