.PHONY: all build test lint clean docker-up docker-down deploy help

BIN := bin/app
ECHO := @echo

all: build test ## Build and test

## Build the application binary
build: deps
	go build -o $(BIN) ./cmd/app

test: ## Run the test suite
	go test ./...

test-integration:
	@# one process per package: the tests mutate the work directory
	$(foreach t,$(INTEGRATION_TESTS),$(GO_TEST) -run $(t);)

test-coverage:
	@# coverage per package
	$(foreach p,$(PACKAGES),$(GO_TEST) -cover $(p);)

lint:
	golangci-lint run

deps:
	go mod download

clean: ## Remove build artifacts
	rm -rf bin dist

docker-up: ## Start local services
	docker compose up -d

docker-down:
	docker compose down -v

deploy: build
	kubectl apply -f k8s/

_internal:
	@echo internal

version: ## Print the binary name
	$(ECHO) "app $(BIN)"

$(BIN): build

%.o: %.c
	$(CC) -c $<

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST)

# Image and release helpers
include mk/docker.mk
-include local.mk
include $(EXTRA_MK)
