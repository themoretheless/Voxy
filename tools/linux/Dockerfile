FROM rustlang/rust:nightly-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libasound2-dev \
    libvulkan1 mesa-vulkan-drivers libegl1 libegl-mesa0 libgl1-mesa-dri \
    xvfb xauth libx11-6 libxcursor1 libxrandr2 libxi6 libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 \
    && rm -rf /var/lib/apt/lists/*
ENV XDG_RUNTIME_DIR=/tmp/voxy-runtime
RUN mkdir -p /tmp/voxy-runtime && chmod 700 /tmp/voxy-runtime
