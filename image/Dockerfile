FROM public.ecr.aws/docker/library/rust:1.57 AS builder
WORKDIR queue-consumer
COPY ./queue-consumer .
RUN cargo build  --release --bin queue-consumer
CMD ["./target/release/queue-consumer"]