#!/bin/bash
set -e

. "$(dirname "$0")/mangle_settings.sh"

ffmpeg -i "$1" -b:v $MANGLE_VIDEO_BITRATE -b:a $MANGLE_AUDIO_BITRATE -vf "$MANGLE_VIDEO_FILTER" -af "$MANGLE_AUDIO_FILTER" mangled.mp4
