#!/bin/bash

case $1 in

  start)
    docker compose -f docker-compose.yaml up --build -V -d --force-recreate
    ;;

  stop)
    docker compose -f docker-compose.yaml down
    ;;

  *)
    echo -n "usable options are start|stop"
    ;;
esac



