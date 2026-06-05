# Lab4 Third-Party Dependencies

This directory is reserved for third-party source pointers.

Use a Git submodule for `llama.cpp`:

```bash
git submodule add https://github.com/ggml-org/llama.cpp lab4/third_party/llama.cpp
git submodule update --init --recursive
```

Do not copy vendored source snapshots into this directory manually. Do not
commit build outputs.
