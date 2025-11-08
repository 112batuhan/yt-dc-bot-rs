FROM rust:latest as rust-builder
WORKDIR /usr/src/yt-dc-bot
RUN apt update
RUN apt install -y libopus-dev
COPY . .
RUN cargo build --release

FROM rust:slim
ARG DISCORD_TOKEN
ARG DATABASE_URL
WORKDIR /usr/src/yt-dc-bot
RUN apt update
RUN apt install -y libopus-dev
RUN apt install -y curl
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o ./yt-dlp
RUN chmod a+rx ./yt-dlp 
COPY --from=rust-builder /usr/src/yt-dc-bot/target/release/yt-dc-bot .
ENV DISCORD_TOKEN=${DISCORD_TOKEN}
ENV DATABASE_URL=${DATABASE_URL}
ENTRYPOINT ["./yt-dc-bot"]
