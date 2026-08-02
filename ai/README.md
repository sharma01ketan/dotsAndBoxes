# dab-ai

Python training package for the Dots and Boxes AlphaZero pipeline.

Phase 4 will add PyTorch/MPS self-play, WebGPU-backed rollouts, and ONNX export.
This directory is a scaffold only (KET-5.3).

## Setup

With [uv](https://github.com/astral-sh/uv):

```bash
cd ai
uv sync
uv run dab-ai
uv run pytest
```

With pip:

```bash
cd ai
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
dab-ai
pytest
```
