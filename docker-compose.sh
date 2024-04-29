#!/bin/bash

case $1 in

  start)
    docker compose -f docker-compose.yaml up --build -V -d --force-recreate
    ;;

  stop)
    docker compose -f docker-compose.yaml down
    ;;

  restart)
    docker compose -f docker-compose.yaml down
    sleep 5
    docker compose -f docker-compose.yaml up --build -V -d --force-recreate
    ;;

  *)
    echo -n "usable options are start|stop|restart"
    ;;
esac



