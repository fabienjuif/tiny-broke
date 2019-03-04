FROM clux/muslrust as builder

RUN apt-get update \
  && apt-get install -y libzmq3-dev \
  &&   rm -rf /var/lib/apt/lists/*

ENV RUSTFLAGS "-C opt-level=s"
COPY . /volume/
RUN cargo build --release

FROM scratch

WORKDIR /repo

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/zeromq-rs /zeromq-rs

ENTRYPOINT ["/zeromq-rs"]
CMD [""]
