# Dockerfile for creating a statically-linked Rust application using docker's
# multi-stage build feature. This also leverages the docker build cache to avoid
# re-downloading dependencies if they have not changed.
FROM rust:1.52.1 AS build

# Setup dummy project
WORKDIR /usr/src/app
RUN USER=root cargo init

# Copy cargo files and build deps
COPY Cargo.toml ./
RUN cargo build --release

# Copy the source and build the application.
COPY src ./src
COPY sqlx-data.json ./
RUN cargo build --release
RUN cargo test --release

# Copy the statically-linked binary into a scratch container.
FROM gcr.io/distroless/cc
COPY --from=build /usr/src/app/target/release/founder-leaderboard .
USER 1000
CMD ["./founder-leaderboard"]
