FROM rust:latest as rust-builder
WORKDIR /usr/src/yt-dc-bot
RUN apt update && apt install -y libopus-dev
COPY . .
RUN cargo build --release

FROM rust:slim
ARG DISCORD_TOKEN
ARG DATABASE_URL
WORKDIR /usr/src/yt-dc-bot
RUN apt update && apt install -y curl python3 libopus-dev
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp
COPY --from=rust-builder /usr/src/yt-dc-bot/target/release/yt-dc-bot .
ENV DISCORD_TOKEN=${DISCORD_TOKEN}
ENV DATABASE_URL=${DATABASE_URL}
ENTRYPOINT ["./yt-dc-bot"]
