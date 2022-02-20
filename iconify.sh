#!/bin/bash

echo "data:image/png;base64,`cat $1 | base64 -w 0`"
