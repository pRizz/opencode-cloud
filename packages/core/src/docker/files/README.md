This directory contains files copied into the Docker image via `Dockerfile` `COPY` lines.
When adding new files here, update the minimal build context in
`packages/core/src/docker/image.rs` (see `create_build_context` and its helper)
so `occ`/CLI builds include the new assets.
