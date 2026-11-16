#!/bin/sh
set -eu
ip route replace default via "$1"
exec sleep infinity
