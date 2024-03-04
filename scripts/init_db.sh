#!/usr/bin/env bash
set -x
set -eo pipefail


# Launch postgres using Docker
if [[ -z "$SKIP_DOCKER" ]]
then
  docker run \
    -e POSTGRES_USER=postgres \
    -e POSTGRES_PASSWORD=password \
    -e POSTGRES_DB=postgres_db \
    -p "5432":5432 \
    -d postgres \
    postgres -N 1000
    # ^ Increased maximum number of connections for testing purposes
fi

>&2 echo "Postgres is running in Docker"