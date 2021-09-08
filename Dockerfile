FROM ekidd/rust-musl-builder:stable as builder
RUN USER=root cargo new --bin chainstate
WORKDIR /home/rust/src/chainstate
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs
ADD src ./src/
RUN rm ./target/x86_64-unknown-linux-musl/release/deps/chainstate*
RUN cargo build --release

FROM alpine:latest
EXPOSE 8000
ENV TZ=Etc/UTC \
    APP_USER=appuser
RUN addgroup -S $APP_USER && adduser -S -g $APP_USER $APP_USER
COPY --from=builder /home/rust/src/chainstate/target/x86_64-unknown-linux-musl/release/chainstate /usr/src/app/chainstate
RUN chown -R $APP_USER:$APP_USER /usr/src/app
USER $APP_USER
WORKDIR /usr/src/app
ENTRYPOINT /usr/src/app/chainstate