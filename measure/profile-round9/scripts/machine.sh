#!/usr/bin/env bash
# The state of the machine at a group's start or end.
echo "## machine $1 $(date -u +%FT%TZ)"
echo "nproc: $(nproc)"
grep -m1 'model name' /proc/cpuinfo
free -m | sed -n '1,2p'
echo "loadavg: $(cat /proc/loadavg)"
