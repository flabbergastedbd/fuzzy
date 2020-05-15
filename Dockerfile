FROM rust:slim

WORKDIR /usr/src/fuzzy
COPY . .

RUN rustup component add rustfmt
RUN apt-get --yes update && apt-get --yes install libpq-dev
RUN cargo install --release --path /usr/local/bin
