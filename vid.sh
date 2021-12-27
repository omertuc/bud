#!/bin/bash

set -euxo pipefail

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")

ffmpeg -framerate 60 -pattern_type glob -i '*.png' -b:v 5000000 out.mp4
