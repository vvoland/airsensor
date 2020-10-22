#!/bin/bash

execute() {
    echo "\$ $1"
    ssh pi@raspberrypi.local "$1"
}
case $1 in
    "stop")
        execute "gpio -g write 25 0"
        ;;
    "start")
        execute "gpio -g write 25 1"
        ;;
    "fast")
        execute "sudo avrdude -p m328p -P /dev/spidev0.0 -c linuxspi -b 1000 -U lfuse:w:0xE2:m"
        make clean
        ;;
    "slow")
        execute "sudo avrdude -p m328p -P /dev/spidev0.0 -c linuxspi -b 1000 -U lfuse:w:0x62:m"
        make clean
        ;;
    "flash")
        ./flash.sh "$2"
        ;;
    *)
        echo "Invalid command" >&2
        exit 1
        ;;
esac
