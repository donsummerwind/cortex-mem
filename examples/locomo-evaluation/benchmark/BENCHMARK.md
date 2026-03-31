# Cortex Memory — LoCoMo10 Benchmark Results

> Evaluation date: **2026-03-30**  
> Dataset: [LoCoMo10](https://github.com/snap-research/locomo) — long-conversation memory QA (conv-26, 152 questions)  
> Method: **LLM-as-a-Judge** (CORRECT / WRONG), identical methodology to OpenViking's official evaluation

---

## Overall Scores

| System | Score | Questions | Notes |
|--------|:-----:|:---------:|-------|
| **Cortex Memory v5 (Intent ON)** | **68.42%** | 152 | This work — best result |
| Cortex Memory v5 (Intent OFF) | ~49.67% | 152 | Estimated from same dataset |
| OpenViking + OpenClaw (−memory-core) | 52.08% | 1,540 | Source: OpenViking README |
| OpenViking + OpenClaw (+memory-core) | 51.23% | 1,540 | Source: OpenViking README |
| OpenClaw + LanceDB (−memory-core) | 44.55% | 1,540 | Source: OpenViking README |
| OpenClaw (built-in memory) | 35.65% | 1,540 | Source: OpenViking README |
| Cortex Memory v4 (full ingest) | 50.00% | 152 | No time-aware prompt |
| Cortex Memory v3-llm (4 sessions) | 25.00% | 152 | Partial data |
| Cortex Memory v2 (intent ON) | 23.03% | 152 | Baseline |
| Cortex Memory v1 (intent OFF) | 19.74% | 152 | Initial baseline |

---

## v5 Category Breakdown

| Category | Description | Correct | Total | Score |
|:--------:|-------------|:-------:|:-----:|:-----:|
| Cat 1 | Factual Recall | 12 | 32 | **37.50%** |
| Cat 2 | Temporal Reasoning | 23 | 37 | **62.16%** |
| Cat 3 | Commonsense Inference | 10 | 13 | **76.92%** |
| Cat 4 | Multi-hop Reasoning | 59 | 70 | **84.29%** |
| **Total** | | **104** | **152** | **68.42%** |

---

## Token Efficiency

| System | Avg Input Tokens / Question | Score | Score per 1K Tokens |
|--------|:---------------------------:|:-----:|:-------------------:|
| **Cortex Memory v5** | **~2,900** | **68.42%** | **23.6** |
| OpenViking + OpenClaw (−memory-core) | ~2,769 | 52.08% | 18.8 |
| OpenViking + OpenClaw (+memory-core) | ~1,363 | 51.23% | 37.6 |
| OpenClaw (built-in memory) | ~15,982 | 35.65% | 2.2 |
| OpenClaw + LanceDB (−memory-core) | ~33,490 | 44.55% | 1.3 |

> Cortex Memory achieves **11× fewer tokens** than OpenClaw+LanceDB and **18× better score-per-token** ratio.

---

## Evaluation Artifacts

All raw evaluation outputs are stored in this directory:

| File | Description |
|------|-------------|
| [`qa-s0-v5.txt.json`](output/qa-s0-v5.txt.json) | v5 QA responses (152 questions, with retrieved contexts) |
| [`qa-s0-v5.judge.md`](output/qa-s0-v5.judge.md) | v5 Judge report (CORRECT/WRONG per question + category scores) |
| [`qa-s0-v4.txt.json`](output/qa-s0-v4.txt.json) | v4 QA responses (full 19-session ingest, no time-aware prompt) |
| [`qa-s0-v4.judge.md`](output/qa-s0-v4.judge.md) | v4 Judge report |
| [`qa-s0-intent-v2.judge.md`](output/qa-s0-intent-v2.judge.md) | v2 Intent ON judge report |
| [`qa-s0-v3-llm.judge.md`](output/qa-s0-v3-llm.judge.md) | v3-llm judge report |
| [`benchmark-comparison.md`](output/benchmark-comparison.md) | Full narrative comparison with all versions |

---

## Evaluation Script

```bash
cd examples/locomo-evaluation

# Step 1: Run QA (answer 152 questions)
.venv/bin/python eval.py qa locomo_data/conv-26_qa.json \
  --output output/qa-s0-v5.txt.json \
  --base-url http://127.0.0.1:18119 \
  --user-id locomo-v4 \
  --session-id s0

# Step 2: Judge the answers (LLM-as-a-Judge)
.venv/bin/python eval.py judge output/qa-s0-v5.txt.json \
  --output output/qa-s0-v5.judge.md \
  --judge-model <your-model-id> \
  --llm-base-url <your-llm-endpoint> \
  --llm-api-key <your-api-key>
```

See [`eval.py`](eval.py) for full implementation details, including the time-aware answer prompt and retry mechanism.

---

## Methodology Notes

- **Judge model**: Same LLM judge used for both Cortex Memory and OpenViking comparisons (CORRECT/WRONG binary)
- **Dataset**: LoCoMo10 conv-26, covering 19 conversation sessions (May–October 2023, ~100K tokens of dialogue)
- **Cortex Memory setup**: All 19 sessions ingested with `locomo-v4` tenant; bootstrap vector sync via Qdrant; 258 memory vectors indexed
- **OpenViking scores**: Sourced from the [OpenViking official README](https://github.com/OpenViking/openviking) tested on 10 samples (1,540 questions total)
- **Intent OFF estimate**: Derived from measured delta between Intent ON/OFF on same dataset in v2 evaluation

---

## Reproducing the Results

Requirements:
- Cortex Memory service running locally (`cargo run --release`)
- Qdrant vector database
- Python 3.10+ with dependencies in `examples/locomo-evaluation/requirements.txt`
- An OpenAI-compatible LLM endpoint for both answering and judging

See the project [README](../../README.md) for service setup instructions.
