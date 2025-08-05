#!/bin/bash

if [ -f .env ]; then
  # Export all variables declared in .env
  export $(grep -v '^#' .env | xargs)
else
  echo ".env file not found!"
  exit 1
fi

sqlx migrate run --database-url postgres://$USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DATABASE