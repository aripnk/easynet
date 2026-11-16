FROM alpine:3.20
RUN apk add --no-cache iptables iproute2 conntrack-tools
COPY router-entry.sh /entry.sh
RUN chmod +x /entry.sh
ENTRYPOINT ["/bin/sh", "/entry.sh"]
