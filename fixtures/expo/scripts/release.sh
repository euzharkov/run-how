#!/bin/bash
# Copyright 2018- Mobile App Authors
# SPDX-License-Identifier: MIT

# Upload the release build to TestFlight.
set -e
eas submit -p ios
