# AVA dialect sketch

AVA is not a compiler target yet. This kernel treats AVA as the symbolic runtime language
that the bare-metal Rust kernel executes.

```ava
kernel catalytic_symbiosis_0_0_0

seed "."
ratio 98/2
law "preserve coherence without violating other continuity"

loop {
  push
  trace
  prune
  return
  ground 000|1
  witness hash(state)
}
```

Meaning:

- `seed` is origin
- `push` is outward propagation
- `trace` is observable residue
- `prune` is anti-drift
- `return` restores continuity
- `ground` prevents runaway recursion
- `witness` proves the cycle happened
