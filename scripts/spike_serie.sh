#!/usr/bin/env bash
# PROTOTYPE : lance une série du Spike, un run à la fois pour ne pas fausser le RTF.
# Usage : scripts/spike_serie.sh <config> "<extraits>" <options du banc...>
set -u
cd /workspaces/transcripteur
BIN=${SPIKE_BIN:-target/release}
export LD_LIBRARY_PATH=$PWD/$BIN
config=$1
extraits=$2
shift 2
mkdir -p data/raw/spike
for e in $extraits; do
    sortie=data/raw/spike/$config--$e.jsonl
    [ -f "$sortie" ] && continue
    ./$BIN/spike --wav data/raw/corpus/$e.wav --sortie "$sortie.tmp" "$@" \
        && mv "$sortie.tmp" "$sortie"
    echo "$(date +%H:%M:%S) $config $e rc=$?"
done
