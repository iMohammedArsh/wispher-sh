#!/usr/bin/env bash
# Renders benchmark clips for `quiet_speech_bench` (crates/hams-core/src/stt/whisper.rs).
# Each sentence becomes <name>.wav (16 kHz, 16-bit mono) plus <name>.txt (the reference).
# `say -v Whisper` is macOS's whispering voice; it is a stand-in, not a human whisper,
# so drop real recordings (name.wav + name.txt) into the same folder as well.
#
#   scripts/make-quiet-fixtures.sh [out_dir]      # default: /tmp/hams-bench
#   HAMS_BENCH_DIR=/tmp/hams-bench HAMS_BENCH=1 cargo test -p hams-core --release \
#       quiet_speech_bench -- --ignored --nocapture
set -euo pipefail

out="${1:-/tmp/hams-bench}"
mkdir -p "$out"

sentences=(
  "Please send the report to the design team before noon tomorrow."
  "I think we should move the meeting to Thursday afternoon."
  "Remind me to buy milk, eggs and bread on the way home."
  "The new keyboard feels much faster than the old one."
  "Can you check whether the invoice has been paid yet."
  "Let us review the budget again once the numbers are final."
  "I will call the dentist on Monday to move my appointment."
  "The train to the airport leaves from platform four at nine."
  "Could you forward that email to everyone on the project."
  "We need a quieter place to talk about the contract."
  "Tell her I will be about ten minutes late for dinner."
  "Please turn off the lights when you leave the office tonight."
  "The package should arrive on Wednesday if nothing goes wrong."
  "I would like a coffee with a little bit of milk please."
  "She said the new phone has a much better camera."
  "Add a reminder to renew the car insurance next week."
  "Let me know if the printer on the second floor is working."
  "He finished reading the book and returned it to the library."
)
voices=("Samantha:normal" "Whisper:whisper")

i=0
for sentence in "${sentences[@]}"; do
  i=$((i + 1))
  for pair in "${voices[@]}"; do
    voice="${pair%%:*}"
    tag="${pair##*:}"
    name="$out/${tag}-$i"
    say -v "$voice" -o "$name.wav" --data-format=LEI16@16000 "$sentence"
    printf '%s\n' "$sentence" > "$name.txt"
  done
done
echo "Wrote $((i * 2)) clips to $out"
