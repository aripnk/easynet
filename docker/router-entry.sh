#!/bin/sh
set -eu
MODE=$1
CIDR=$2
WAN=$(ip -o -4 addr show | awk '/172\.30\.0\./ {print $2; exit}')
LAN=$(ip -o -4 addr show | awk '/10\.10\./ {print $2; exit}')
if [ -z "$WAN" ] || [ -z "$LAN" ]; then
  echo "router missing wan or lan address" >&2
  exit 1
fi
case "$MODE" in
  cone)
    iptables -t nat -A POSTROUTING -s "$CIDR" -o "$WAN" -j MASQUERADE
    ;;
  restricted)
    iptables -t nat -A POSTROUTING -s "$CIDR" -o "$WAN" -j MASQUERADE
    iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
    iptables -A FORWARD -i "$WAN" -o "$LAN" -j DROP
    ;;
  symmetric)
    iptables -t nat -A POSTROUTING -s "$CIDR" -o "$WAN" -j MASQUERADE --random-fully
    iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
    iptables -A FORWARD -i "$WAN" -o "$LAN" -j DROP
    ;;
  *)
    echo "bad mode $MODE" >&2
    exit 1
    ;;
esac
exec sleep infinity
