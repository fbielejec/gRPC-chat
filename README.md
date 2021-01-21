# gRPC-redis-pubsub

ping-pong + chat server using gRPC and proto3 with Rust.
Chat server uses redis pub-sub for dispatching messages to clients connected to different instances of the service.

## start backend service (redis)

```bash
docker-compose -f docker-compose.yml up
```

## watch, build and run

```bash
cargo watch -x "run -- --bin server"
```

## connect a [grpcurl](https://github.com/fullstorydev/grpcurl) client to the running server

*Streaming* : connect as users with id "filip" and "juan" (using "from" header), read messages from stdin and stream them to the connected clients:

```bash
grpcurl -plaintext -proto proto/chat.proto -d @ -H 'from: filip' localhost:50051 chat.Chat/Chat

grpcurl -plaintext -proto proto/chat.proto -d @ -H 'from: juan' localhost:50051 chat.Chat/Chat
```

Messages are routed using the "to" field, e.g. this message will end up in the stdout of the user "filip" when sent form stdin of user "juan":

```json
{"to": "filip","message":"Hi"}
```

*Streaming* : send one message from user id "juan" to the chat endpoint, message gets picked up by the user "filip":

```bash
grpcurl -plaintext -import-path ./proto -proto chat.proto -H 'from: juan' -d '{"to": "filip", "message": "hello!"}' localhost:50051 chat.Chat/Chat
```

Invoke the ping-pong endpoint:

```bash
grpcurl -plaintext -import-path ./proto -proto pingpong.proto localhost:50051 pingpong.PingPong/SendPing
```
