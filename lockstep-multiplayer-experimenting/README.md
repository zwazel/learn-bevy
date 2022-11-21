# Lockstep

## Run Server

Server with name "ImTheServer", max 2 clients, default port and host, 50 Tickrate (The lower, the faster), and not
saving replays
`cargo run --bin main server ImTheServer 2 . . 50 false`

## Run Client

Client with name "ImTheClient", default port and host, 50 Tickrate (The lower, the faster), and not saving replays
`cargo run --bin main . ImTheClient . . . 50 false`
