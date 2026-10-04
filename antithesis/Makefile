CONTAINER_CMD ?= $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null || echo "docker")
ROOT = ../
REPO := us-central1-docker.pkg.dev/molten-verve-216720/honey-pwhale-repository

.PHONY: drivers config
drivers:
	$(CONTAINER_CMD) build -f drivers/Dockerfile $(ROOT) -t $(REPO)/dbdb-drivers:latest

config:
	$(CONTAINER_CMD) build config/ -t $(REPO)/dbdb-config:latest

.PHONY: down run push
down:
	docker-compose -f config/docker-compose.yaml down -v

run: down
	docker-compose -f config/docker-compose.yaml up

push:
	$(CONTAINER_CMD) push $(REPO)/dbdb-drivers:latest $(REPO)/dbdb-config:latest