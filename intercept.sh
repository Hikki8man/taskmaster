#!/bin/bash
while true
do
    trap 'echo yo' INT
	# exit 1
	# echo "Press [CTRL+C] to stop.."
	sleep 1
done
