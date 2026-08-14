FROM docker.io/library/debian:stable-slim

COPY dist/plumb /usr/local/bin/plumb

ENTRYPOINT ["/usr/local/bin/plumb"]
