#!/bin/bash
set -e

if [ "$MANGLE_FPS" = "" ] ; then
	export MANGLE_FPS=30
fi
if [ "$MANGLE_SCALE" = "" ] ; then
	export MANGLE_SCALE=640
fi
if [ "$MANGLE_NOISE_AMOUNT" = "" ] ; then
	export MANGLE_NOISE_AMOUNT=50
fi
if [ "$MANGLE_VIDEO_BITRATE" = "" ] ; then
	export MANGLE_VIDEO_BITRATE=250k
fi
if [ "$MANGLE_AUDIO_BITRATE" = "" ] ; then
	export MANGLE_AUDIO_BITRATE=1k
fi

echo FPS: $MANGLE_FPS
echo SCALE: $MANGLE_SCALE
echo NOISE_AMOUNT: $MANGLE_NOISE_AMOUNT
echo VIDEO_BITRATE: $MANGLE_VIDEO_BITRATE
echo AUDIO_BITRATE: $MANGLE_AUDIO_BITRATE

export MANGLE_VIDEO_FILTER="scale=$MANGLE_SCALE:-1,fps=$MANGLE_FPS,noise=c0s=$MANGLE_NOISE_AMOUNT:allf=t+u,unsharp=13:13:5"
export MANGLE_AUDIO_FILTER="volume=100"