FROM dart:3.12.1-sdk@sha256:df748054c761938f24bae1e3876b0a29dc0b7cfaecbb0dd0883af5d2c3c076a1

RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 clang binutils

# Only the disposable fault-test engine comes from the signed newer repository.
COPY sqlite_linux.sources /etc/apt/sources.list.d/dust-sqlite.sources
RUN apt-get update && apt-get install -y --no-install-recommends -t trixie \
    libsqlite3-0=3.53.4-2 libsqlite3-dev=3.53.4-2

ENTRYPOINT ["python3", "/source/scripts/sqlite_linux_runner.py"]
