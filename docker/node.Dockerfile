FROM alpine:3.20
RUN apk add --no-cache iproute2
COPY node-entry.sh /entry.sh
RUN chmod +x /entry.sh
ENTRYPOINT ["/bin/sh", "/entry.sh"]
