# Packages the prebuilt static Linux binary on a tiny base that still has a
# shell, so the image can be used the normal way: declared as a CI job image
# (GitLab `image:`, GitHub `container:`) whose script runs `markdown-style ...`,
# or run directly with a mount.
#
# The binary is built by the release workflow.
# Place the extracted `markdown-style` next to this file before building:
#
#   docker build -t markdown-style .
#   docker run --rm -v "$PWD:/work" markdown-style
#
FROM busybox:stable-musl

# Links the ghcr.io package to this repository.
LABEL org.opencontainers.image.source="https://github.com/very-good-software-ltd/markdown-style"

COPY markdown-style /usr/local/bin/markdown-style

WORKDIR /work

# Run directly with no arguments to lint the mounted directory; override the
# command (for example `format .`) to do something else.
ENTRYPOINT ["markdown-style"]
CMD ["lint", "."]

