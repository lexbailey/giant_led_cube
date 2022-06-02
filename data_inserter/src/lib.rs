use reqwest::StatusCode;
use twapi_reqwest::v1;
use worker::*;

mod schema;
mod utils;
use schema::*;
use utils::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    utils::log_request(&req);
    utils::set_panic_hook();
    let config = Config::from_env(&env);

    Router::with_data(config)
        .post_async("/", |mut req, ctx| async move {
            if req.headers().get("Authorization").unwrap_or(None)
                != Some(format!("Bearer {}", ctx.data.cube_secret.clone()))
            {
                let public_error = "did not provide required Authorization header";
                console_log!("{}", public_error);
                return Response::error(public_error, 500);
            }

            let datapoint: Datapoint = match req.json().await {
                Ok(datapoint) => datapoint,
                Err(e) => {
                    let public_error = format!("error parsing input as datapoint: {}", e);
                    console_log!("{}: {}", public_error, e);
                    return Response::error(public_error, 500);
                }
            };

            let mut clickhouse_public_error = None;
            if let Err(e) = datapoint.insert_to_clickhouse(&ctx.data.clickhouse).await {
                let public_error = format!("error writing datapoint to clickhouse");
                console_log!("{}: {}", public_error, e);
                clickhouse_public_error = Some(public_error);
            } else {
                console_log!("datapoint written to clickhouse successfully");
            }

            if let Datapoint::GameSolve(game_solve) = datapoint {
                let seconds: u32 = game_solve.play_time_milliseconds / 1000;
                let minutes = seconds / 60;
                let elapsed;
                if minutes > 1 {
                    elapsed = format!("{} minutes and {} seconds", minutes, seconds % 60);
                } else {
                    elapsed = format!("{} seconds", seconds);
                }

                let tweet = if game_solve.new_top_score {
                    format!(
                        "NEW TOP SCORE! ⭐ Someone solved @giant_cube in just {}",
                        elapsed
                    )
                } else {
                    format!(
                        "🎉 Someone solved @giant_cube in {}! Think you can do better?",
                        elapsed
                    )
                };

                let a = vec![];
                let b = vec![("status", &*tweet)];
                let tweet_result = v1::post(
                    "https://api.twitter.com/1.1/statuses/update.json",
                    &a,
                    &b,
                    &ctx.data.twitter.consumer_key,
                    &ctx.data.twitter.consumer_secret,
                    &ctx.data.twitter.access_key,
                    &ctx.data.twitter.access_secret,
                )
                .await;
                if let Err(e) = tweet_result {
                    let public_error = format!("error tweeting datapoint");
                    console_log!("{}: {}", public_error, e);
                    return Response::error(public_error, 500);
                }
                let tweet_result = tweet_result.unwrap();
                if tweet_result.status() != StatusCode::OK {
                    let public_error = format!("error tweeting datapoint");
                    console_log!(
                        "{}: status {}: {:?}",
                        public_error,
                        tweet_result.status(),
                        tweet_result.text().await
                    );
                    return Response::error(public_error, 500);
                }
                console_log!("game solve datapoint tweeted successfully: {:?}", tweet);
            }

            if let Some(clickhouse_public_error) = clickhouse_public_error {
                Response::error(clickhouse_public_error, 500)
            } else {
                Response::ok("datapoint handled successfully")
            }
        })
        .get("/version", |_, _| {
            let name = env!("CARGO_PKG_NAME");
            let version = env!("CARGO_PKG_VERSION");
            Response::ok(format!("{} {}", name, version))
        })
        .run(req, env)
        .await
}
