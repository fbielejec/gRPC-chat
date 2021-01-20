FROM debian:stretch-slim
MAINTAINER "Filip Bielejec" <filip@clashapp.co>

EXPOSE 3001

WORKDIR api

COPY target/release/server /api/server

ENTRYPOINT ["./server"]
