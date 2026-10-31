# Real-model result provenance

`summary.json` is the aggregate used by the real-model results table in the preprint. It is derived from `benchmark-01.json`, `benchmark-02.json`, and `benchmark-03.json` by `../scripts/aggregate_results.py`.

Verify every aggregate statistic:

```bash
python3 experiments/real-gguf-qwen2.5-0.5b/scripts/aggregate_results.py --check
```

The command recomputes cold-start count, warm-request count, failures, medians, minima, maxima, and sample standard deviation from the three raw reports. It fails if `summary.json` differs in any value. No statistic is edited manually.
