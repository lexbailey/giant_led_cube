use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use worker::*;

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        pub use self::console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}

pub fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub cube_secret: String,
    pub clickhouse: ClickhouseConfig,
    pub twitter: TwitterConfig,
}

impl Config {
    pub fn from_env(env: &Env) -> Config {
        Config {
            cube_secret: unwrap_env_secret(env, "CUBE_SECRET"),
            clickhouse: ClickhouseConfig::from_env(env),
            twitter: TwitterConfig::from_env(env),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClickhouseConfig {
    pub url: String,
    pub user: String,
    pub password: String,
}

impl ClickhouseConfig {
    pub fn from_env(env: &Env) -> ClickhouseConfig {
        ClickhouseConfig {
            url: unwrap_env_secret(env, "CLICKHOUSE_URL"),
            user: unwrap_env_secret(env, "CLICKHOUSE_USER"),
            password: unwrap_env_secret(env, "CLICKHOUSE_PASSWORD"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TwitterConfig {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_key: String,
    pub access_secret: String,
}

impl TwitterConfig {
    pub fn from_env(env: &Env) -> TwitterConfig {
        TwitterConfig {
            consumer_key: unwrap_env_secret(env, "TWITTER_CONSUMER_KEY"),
            consumer_secret: unwrap_env_secret(env, "TWITTER_CONSUMER_SECRET"),
            access_key: unwrap_env_secret(env, "TWITTER_ACCESS_KEY"),
            access_secret: unwrap_env_secret(env, "TWITTER_ACCESS_SECRET"),
        }
    }
}

fn unwrap_env_secret(env: &Env, name: &str) -> String {
    match env.secret(name) {
        Ok(secret) => secret.to_string(),
        Err(e) => panic!("could not fetch secret {:?}: {}", name, e),
    }
}
