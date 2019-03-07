
FROM alpine as builder

RUN apk add --update zeromq-dev cargo rust && rm -rf /var/cache/apk/*

WORKDIR /workdir

ENV RUSTFLAGS "-C opt-level=s"
COPY . .
RUN cargo build --release

FROM alpine

RUN apk add --update libzmq  && rm -rf /var/cache/apk/*

COPY --from=builder /workdir/target/release/tiny-broke /tiny-broke

ENTRYPOINT ["/tiny-broke"]
CMD [""]
