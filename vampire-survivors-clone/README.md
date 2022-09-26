# Run Server

`cargo run --bin server [1] [2] [3]`

1. amount of clients that are allowed to be connected to the server at the same time, default is 4.
2. port number, default is 5000.
3. Host address, default is 127.0.0.1.

To use the default and skip an argument, use "-" as argument.

## Example

`cargo run --bin server 10 5000 localhost`

# Run Client

`cargo run --bin client [1] [2] [3]`

1. username, default is "Player_[timestamp]".
2. Host address, default is "127.0.0.1".
3. port number, default is 5000.

To use the default and skip an argument, use "-" as argument.

## Example

`cargo run --bin client Player_1 localhost 5000`