# podman build  -f daemon/Dockerfile -t daemon-server --target server ./
# podman build  -f daemon/Dockerfile -t daemon-endpoint --target daemon ./

FROM fedora as daemon
COPY /target/release/examples/daemon_mode ./
COPY /daemon/tests/configs/podman.toml ./

CMD ["./daemon_mode", "podman.toml"]

FROM node:current as server
ENV LISTEN="0.0.0.0"
COPY ["/tools/daemon server/build", "./build"]
COPY ["/tools/daemon server/package*.json", "./"]
COPY ["/tools/daemon server/tests", "./tests"]
RUN npm install
CMD ["node", "./build/index.js"]