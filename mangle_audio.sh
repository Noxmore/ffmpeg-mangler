#!/bin/bash
set -e

. "$(dirname "$0")/mangle_settings.sh"

ffmpeg -i "$1" -b:a $MANGLE_AUDIO_BITRATE -sample_fmt s16p mangled.mp3
