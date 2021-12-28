#!/bin/bash

set -euxo pipefail

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")

ffmpeg -framerate 60 -pattern_type glob -i 'frame-*.png' -b:v 50000000 -vf eq=brightness=0.65:saturation=8:contrast=2.5 out.mp4
