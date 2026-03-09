FROM node:20

ARG TZ
ENV TZ="$TZ"

# Tauri build dependencies and networking tools
RUN apt-get update && apt-get install -y --no-install-recommends \
  fzf \
  vim \
  iptables \
  ipset \
  dnsutils \
  aggregate \
  # Tauri build dependencies
  build-essential \
  pkg-config \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  && apt-get clean && rm -rf /var/lib/apt/lists/*

# Install git-delta
ARG GIT_DELTA_VERSION=0.18.2
RUN ARCH=$(dpkg --print-architecture) && \
  wget -q "https://github.com/dandavison/delta/releases/download/${GIT_DELTA_VERSION}/git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
  dpkg -i "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
  rm "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb"

# Create directories with proper permissions
RUN mkdir -p /workspace /home/node/.claude && \
  chown -R node:node /workspace /home/node/.claude

WORKDIR /workspace
