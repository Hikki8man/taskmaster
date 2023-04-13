#!/bin/bash
for i in {1..1000}
do
	echo "tee $i tim"
	exit 1
	sleep 2
done