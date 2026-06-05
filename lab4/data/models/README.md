# Lab4 Models

Put local GGUF model files in this directory.

Large model files are ignored by Git:

- `*.gguf`
- `*.bin`
- `*.safetensors`
- `*.pt`
- `*.pth`

Keep `placeholder.model` only for local smoke tests. Formal Lab4 experiments
must use a real GGUF model and record its name, source, size, and quantization
format in `lab4/reports/deployment.md`.
