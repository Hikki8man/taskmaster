#!/bin/bash
for i in {1..1000}
do
   echo "Yee $i times"
   trap "echo non" INT
   # umask
   # exit 2
   sleep 2
done
