# syntax=docker/dockerfile:1
FROM debian:bookworm-slim

# Install any required OS dependencies for VRE Core (e.g., OpenSSL)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Assume the precompiled VRE binary is placed alongside or downloaded
# For this scaffold, we assume `vre` is available locally or built from source
# COPY vre /usr/local/bin/vre

# Copy compiled VRE bytecode
COPY app.vym /app/app.vym

# Expose standard port for VRE HTTP servers
EXPOSE 8080

# Run VRE Engine
CMD ["vre", "app.vym"]
