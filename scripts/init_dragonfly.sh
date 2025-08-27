#!/usr/bin/env bash
set -x
set -eo pipefail

#if a dragonfly is running, print instructions to kill it and exit
RUNNING_CONTAINER=$(docker ps --filter 'name=dragonfly' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
    echo >&2 "there is a dragonfly container already running, kill it with"
    echo >&2 "  docker kill ${RUNNING_CONTAINER}"
    exit 1
fi

# Launch Dragonfly using docker
docker run \
    -p "6379:6379" \
    -d \
    --name "dragonfly_$(date '+%s')" \
    --ulimit memlock=-1 \
    docker.dragonflydb.io/dragonflydb/dragonfly

>&2 echo "Dragonfly is ready to go!"