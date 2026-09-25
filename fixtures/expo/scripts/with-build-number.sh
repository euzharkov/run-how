#!/bin/bash
# Set the build number, then run the given command.
export BUILD_NUMBER=$(($(cat .build-number)+1))
exec "$@"
