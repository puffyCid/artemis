#!/bin/sh

# Podman required. Runs on localhost
podman run -it -p 9200:9200 -p 9600:9600 -e OPENSEARCH_INITIAL_ADMIN_PASSWORD="Ughsocomplex123567890!" -e "discovery.type=single-node"  --name opensearch-node -d opensearchproject/opensearch:latest