FROM ekidd/rust-musl-builder:stable as builder

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./static ./static
COPY ./src ./src
RUN cargo build --release

FROM alpine:latest

ARG APP=/usr/src/app

EXPOSE 8288

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN addgroup -S $APP_USER \
    && adduser -S -g $APP_USER $APP_USER

RUN apk update \
    && apk add --no-cache ca-certificates tzdata ffmpeg \
    && rm -rf /var/cache/apk/*

COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/swmv ${APP}/swmv

RUN chown -R $APP_USER:$APP_USER ${APP}

RUN mkdir -p ${APP}/media

USER $APP_USER
WORKDIR ${APP}

CMD ./swmv -r -t -p /media

