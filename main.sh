#!/bin/bash

export $(cat .env | grep -v '^#' | xargs)

export DATABASE_URL="postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@$POSTGRES_SERVER/$POSTGRES_DB"

cargo update
#cargo install -j 4 diesel_cli
cargo install diesel_cli

diesel setup
diesel migration run --locked-schema

#cargo run -j 4 --release
cargo run --release
