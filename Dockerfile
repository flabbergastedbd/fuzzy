# docker build -t fuzzy:0.1 .
FROM rust:slim

RUN apt-get --yes update && \
    apt-get --yes install libpq-dev apt-transport-https ca-certificates curl gnupg-agent software-properties-common && \
    rm -rf /var/lib/apt/lists/*

# Needed for diesel
RUN curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add - && \
    add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/debian $(lsb_release -cs) stable" && \
    apt-get --yes update && apt-get --yes install docker-ce-cli=5:18.09.9~3-0~debian-buster && \
    rm -rf /var/lib/apt/lists/*

# Get & compile fuzzy
WORKDIR /usr/src/fuzzy
COPY . .

# Needed for prost build
RUN rustup component add rustfmt && \
    cargo build --release && \
    cp target/release/fuzzy /bin/fuzzy && \
    rm -rf /usr/src/fuzzy

# We mount this as volume
RUN rm -rf /etc/timezone

# Add a fuzzy user just in case
RUN useradd fuzzy

VOLUME /home/fuzzy
WORKDIR /home/fuzzy

USER fuzzy
ENTRYPOINT /bin/fuzzy
