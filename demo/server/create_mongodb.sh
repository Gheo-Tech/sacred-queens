#!/bin/bash
podman rm -f primordial_demo_mongodb
podman run -d \
  --publish 27017:27017 \
  --name primordial_demo_mongodb \
  -d docker.io/library/mongo:latest \
  mongod --bind_ip_all --replSet rs0
