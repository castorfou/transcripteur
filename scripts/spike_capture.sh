#!/usr/bin/env bash
# PROTOTYPE : rejoue des extraits sur un sink nul et les capte par son monitor.
cd /workspaces/transcripteur
M=$(pactl load-module module-null-sink sink_name=spike_nul sink_properties=device.description=spike_nul)
trap 'pactl unload-module $M' EXIT
export LD_LIBRARY_PATH=$PWD/target-vk/release SPIKE_AUDIO_CTX=1000 PULSE_SOURCE=spike_nul.monitor
for e in fr-negatif multilingue; do
    f=data/raw/corpus/$e.wav
    d=$(python3 -c "import wave;w=wave.open('$f');print(int(w.getnframes()/w.getframerate())+1)")
    target-vk/release/spike --capture $((d+5)) --sortie data/raw/spike/cap-n320-vk--$e.jsonl.tmp \
        --seconde whisper --threads 4 --threads-seconde 2 --afficher non 2>/dev/null &
    p=$!
    sleep 4
    paplay --device=spike_nul $f
    wait $p && mv data/raw/spike/cap-n320-vk--$e.jsonl.tmp data/raw/spike/cap-n320-vk--$e.jsonl
    echo "$(date +%H:%M:%S) $e rc=$?"
done
echo CAPTURE_TERMINEE
