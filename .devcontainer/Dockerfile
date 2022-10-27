ARG VARIANT=latest
# Choose between: debian, ubuntu, fedora, opensuse/tumbleweed, opensuse/leap
FROM debian:${VARIANT}
ENV DEBIAN_FRONTEND=noninteractive
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8
ARG CONTAINER_USER=esp
ARG CONTAINER_GROUP=esp

# Ubuntu/Debian
RUN apt-get update \
    && apt-get install -y git python3 python3-pip gcc build-essential curl pkg-config libudev-dev libtinfo5\
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts
RUN adduser --disabled-password --gecos "" ${CONTAINER_USER}

# Fedora
# RUN dnf -y update \
#     && dnf -y install git python3 python3-pip gcc systemd-devel ncurses-compat-libs \
#     && dnf clean all
# RUN adduser ${CONTAINER_USER}

USER ${CONTAINER_USER}
WORKDIR /home/${CONTAINER_USER}

# openSUSE Tumbleweed/Leap
# RUN zypper install -y git gcc libudev-devel ninja python3 python3-pip make \
#     && zypper clean

# Install Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH=${PATH}:/home/${CONTAINER_USER}/.cargo/bin

CMD [ "/bin/bash" ]
