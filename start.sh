#!/bin/sh

while true
do
    if ./dyfi-client; then
        sleep 432000 # five days
    else
        # dyfi-client returned non-zero,
        # probably configuration error, exiting
        exit
    fi
done
