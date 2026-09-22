# Configuration

Load order (fail-closed on invalid config):

1. Compiled defaults  
2. `configs/*.toml`  
3. Environment `TALOS__*` (double-underscore nesting)  

## Files

| File | Purpose |
|------|---------|
| `configs/runtime.toml` | Threads, shutdown grace, max in-flight frames |
| `configs/decision.toml` | `auto_approve_min`, `secondary_min`, hard-fail flags |
| `configs/observability.toml` | Log level, JSON logs, plate redaction |
| `configs/pipeline.toml` | Stage backends (`fixture` / `ai_api` / `local_gpu_ext`) |

## Examples

```bash
# Point at config directory
export TALOS_CONFIG_DIR=./configs

# Override a nested key
export TALOS__DECISION__AUTO_APPROVE_MIN=0.95
```

## Validation rules

- Confidence thresholds ∈ `[0.0, 1.0]`
- `secondary_min < auto_approve_min`
- Secrets never committed in TOML (use env / secret manager for M03/M12)
