# docker build -t fuzzy:0.1 .
FROM rust:slim

WORKDIR /usr/src/fuzzy
COPY . .

# Needed for prost build
RUN rustup component add rustfmt

# Needed for diesel postgres
RUN apt-get --yes update && apt-get --yes install libpq-dev

# Needed for migrations
#RUN cargo install diesel_cli --no-default-features --features "postgres"

# Needed for why we are doing all the above
RUN cargo build --release

# Need to have ca cert, server cert etc.. present here
VOLUME /opt/fuzzy
WORKDIR /opt/fuzzy
ENTRYPOINT /usr/src/fuzzy/target/release/fuzzy
