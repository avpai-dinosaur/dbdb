# The implementation under test: any sibling directory satisfying CONTRACT.md.
# Nothing else in this directory names a language.
IMPL ?= dbdb-cpp

CONTAINER_CMD ?= $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null || echo "docker")
ROOT = ..
REPO := us-central1-docker.pkg.dev/molten-verve-216720/honey-pwhale-repository

IMPL_IMAGE := $(IMPL):latest
HARNESS_IMAGE := $(REPO)/dbdb-drivers:latest

.PHONY: all impl drivers down run push
all: drivers

# The implementation image: an artifact carrier publishing /out. Never run.
impl:
	$(CONTAINER_CMD) build -t $(IMPL_IMAGE) $(ROOT)/$(IMPL)

# The harness image, built on top of it. If your builder insists on pulling
# IMPL_IMAGE from a registry instead of using the local one, build with
# DOCKER_BUILDKIT=0, or load the implementation image into the builder first.
drivers: impl
	$(CONTAINER_CMD) build --build-arg IMPL_IMAGE=$(IMPL_IMAGE) -t $(HARNESS_IMAGE) .

down:
	docker-compose -f config/docker-compose.yaml down -v

run: down
	docker-compose -f config/docker-compose.yaml up

push:
	$(CONTAINER_CMD) push $(HARNESS_IMAGE)
