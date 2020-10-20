#!/bin/bash
set -e

[ ! -f "$1" ] && echo "Invalid file $1" >&2 && exit 1
short=$(basename "$1")
scp "$1" "pi@raspberrypi.local:~/$short"
ssh "pi@raspberrypi.local" "gpio -g mode 25 out; gpio -g write 25 0; sudo avrdude -p m328p -P /dev/spidev0.0 -c linuxspi -b 80000 -e -U flash:w:$short hex && gpio -g write 25 1"
