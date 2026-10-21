# Companion foundation image; no engines or datasets. Release base digest/SBOM
# approval belongs to the release gate, before publishing an image.
FROM rust:1.95-bookworm AS build
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src ./src
COPY crates ./crates
COPY server ./server
RUN cargo build --locked --release -p ironwood-server

FROM debian:bookworm-slim
RUN groupadd --gid 10001 companion && useradd --uid 10001 --gid 10001 --no-create-home companion
RUN mkdir /data && chown 10001:10001 /data && chmod 700 /data
COPY --from=build /source/target/release/ironwood-server /usr/local/bin/ironwood-server
COPY LICENSE /usr/share/doc/ironwood-companion/LICENSE
# Include matching buildable source for the Companion binary, plus the project
# workspace sources needed to resolve its manifest. Never copy build/target data.
COPY Cargo.toml Cargo.lock build.rs LICENSE /usr/share/doc/ironwood-companion/source/
COPY server /usr/share/doc/ironwood-companion/source/server
COPY crates /usr/share/doc/ironwood-companion/source/crates
COPY src /usr/share/doc/ironwood-companion/source/src
USER 10001:10001
EXPOSE 8790
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD ["/usr/local/bin/ironwood-server", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/ironwood-server"]
CMD ["--data-dir", "/data", "--docker"]
