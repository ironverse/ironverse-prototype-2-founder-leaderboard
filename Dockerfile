# Dockerfile for creating a statically-linked Rust application using docker's
# multi-stage build feature. This also leverages the docker build cache to avoid
# re-downloading dependencies if they have not changed.
FROM rust:1.52.1 AS build

# Setup dummy project
WORKDIR /usr/src/app
RUN cargo init

# Copy Cargo.toml and build the dependencies
COPY Cargo.toml ./
RUN cargo build --release

# Copy the source
COPY sqlx-data.json ./
COPY src ./src

# Mark source as updated
RUN touch -a -m ./src/main.rs

# Build the application.
RUN cargo build --release
RUN cargo test --release

# Copy the statically-linked binary into a scratch container.
FROM gcr.io/distroless/cc
COPY --from=build /usr/src/app/target/release/founder-leaderboard .
USER 1000
CMD ["./founder-leaderboard"]
