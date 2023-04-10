#!/bin/bah
while true
do
	trap "echo yoo && exit 1" INT
	sleep 2
	# exit 1
done