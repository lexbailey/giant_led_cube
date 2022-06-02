# giant_led_cube/data_inserter

We send cube activity data to external systems that are easier to update than the cube is. This folder contains a [Cloudflare Worker](https://workers.cloudflare.com) running WASM Rust. The worker does two things with the data right now:

1. Inserts the data into an external ClickHouse database. Anyone can access the read-only data, see [github.com/danieljabailey/giant_led_cube/blob/main/api.md](https://github.com/danieljabailey/giant_led_cube/blob/main/api.md)
2. Tweets from [@giant_cube_bot](https://twitter.com/giant_cube_bot) when the game is solved.
