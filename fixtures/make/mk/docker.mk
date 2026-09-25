image: ## Build the Docker image
	docker build -t app .

include mk/docker.mk
