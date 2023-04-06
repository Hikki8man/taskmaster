#!/bin/bash
while true
do
    trap 'echo "Interrupt signal received."' INT
	# echo "Press [CTRL+C] to stop.."
	sleep 1
done
