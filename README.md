# Daegonica Software twitch_bot

A customizable twitch chatbot.

## Features
- Tracks twitch chat messages/whispers and displays them to the terminal
- Can accept custom commands

## Tech
- Rust

## Status 
Active Development

## How to Run.
Make a new .env file with the following structure:

    CLIENT_ID=id
    CLIENT_SECRET=secret
    TWITCH_CHANNEL=channel

Make a new token.json file with the following structure:
    {"access_token":"token","refresh_token":"secret","created_at":"2025-12-23T16:31:21.342150Z","expires_at":"2025-12-23T20:17:08.342150Z"}

```bash
cargo run
