# Servidor de contas da Tasca. Construir a partir da raiz do repositório:
#   docker build -f deploy/servidor/Dockerfile -t tasca-servidor .
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p tasca-servidor

FROM debian:bookworm-slim
RUN useradd --system --home /dados tasca && mkdir -p /dados && chown tasca /dados
COPY --from=build /src/target/release/tasca-servidor /usr/local/bin/tasca-servidor
USER tasca
ENV TASCA_ENDERECO=0.0.0.0:8080 TASCA_DADOS=/dados/contas.db TASCA_PROXY=1
VOLUME /dados
EXPOSE 8080
CMD ["tasca-servidor"]
