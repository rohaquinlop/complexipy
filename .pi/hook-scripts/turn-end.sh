#!/bin/sh
# pi-hooks: run the gate at the end of each turn that edited files or ran bash.
# pi cannot block, so each failing tree state is reported once.
. .agents/hooks/lib.sh
reported && exit 0
out=$(sh .agents/hooks/gate.sh) && exit 0
mark_reported
printf 'Quality gate failed:\n%s' "$out"
