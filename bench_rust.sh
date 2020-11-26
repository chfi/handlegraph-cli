#!/usr/bin/env bash

GFALIST=$1

timestamp="$(date +"%m%d_%H%M%S")"
JSONFILE="$GFALIST.rust.$timestamp.json"

if [[ -z $2 ]]; then
    WARMUP=""
elif [[ -n $2 ]]; then
    WARMUP="--warmup $2"
fi

INPUTLIST=""

while read line; do
    if [[ -z "$INPUTLIST" ]]; then
        INPUTLIST="\"$line\""
    elif [[ -n "$INPUTLIST" ]]; then
        INPUTLIST="$INPUTLIST,\"$line\""
    fi
done <<<$(cat $GFALIST)

hyperfine $WARMUP -L input "$INPUTLIST" --export-json "$JSONFILE" "./target/release/handlegraph-cli {input}"
