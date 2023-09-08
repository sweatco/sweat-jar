build:
	./scripts/build.sh

build-in-docker:
	./scripts/build-in-docker.sh

dock: build-in-docker

deploy:
	./scripts/deploy.sh

cov:
	./scripts/coverage.sh

test:
	cargo test --package sweat_jar

integration:
	cargo test --package integration-tests

int: integration

fmt:
	cargo +nightly fmt --all
