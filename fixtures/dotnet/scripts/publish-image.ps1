#!/usr/bin/env pwsh
# Publish the API image to the registry
docker build -t shop/api . && docker push shop/api
