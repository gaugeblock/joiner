# Gold yield measurement (GB-213 / GB-225 / GB-227)

> Recorded 2026-10-30. Durable home for the numbers behind the Gold path, because a figure that
> lives only in a chat session has not been recorded (law 12).

## Fixture

`/tmp/gb-gold-crate` - one hole, `grapheme_count`, whose doc comment instructs delegation to
`crate::text::grapheme_count`. The helper exists (it did not until GB-225; see below).

## Yield, four consecutive fills

| Run | Tier | Attempts | Body length |
|:--|:--|:--|:--|
| 1 | Silver | 1 | 258 |
| 2 | **Gold** | 2 | 245 |
| 3 | Silver | 1 | 225 |
| 4 | Silver | 1 | 256 |

4/4 produced a validated candidate; 1/4 was Gold (a repair, therefore a DPO pair). Every body was
real code - none was the `<filled>` placeholder.

**Small sample, stated plainly:** four runs on one fixture establishes that the path works and gives
an order-of-magnitude sense of the Gold rate. It does not establish a rate. Ten runs across three
fixtures would.

## Corpus effect

Before the guards: ingest accepted 36/36 and the DPO shard held 2 rows whose `chosen` was the literal
`<filled>` - pairs that train nothing.

After GB-225 (joiner sends real bodies) and GB-227 (the gate rejects placeholders): ingest accepts
33, rejects 7 by name (`add`, `human_bytes`, `smallest_window`, `grapheme_count`), and the shards are
SFT 16 rows, DPO **1 row with real code**. An empty shard was the honest intermediate state; one
trainable pair is the honest result.

## Defects this measurement exposed, in order

1. `requires_streaming` ignored, so the first request could never succeed (GB-217 step 4).
2. The context dropped the item's doc comment, so the model never saw the guidance (GB-224).
3. Verification wrote the bare candidate as the whole file, erasing `pub mod text;`, so a correct
delegation failed with `cannot find text in the crate root` (GB-225).
4. The fixture's doc comment promised a helper that did not exist, so every earlier attempt
   legitimately reached for `unicode_segmentation` (GB-225).
5. `chosen` was a placeholder because the CLI spooled a literal instead of `FillOutcome::code`
   (GB-225), and the receiving gate accepted it (GB-227).

Each was invisible until the one before it was fixed, which is why the run count matters more than
any single green result.

## The 7 legacy placeholder records: not salvageable, regenerable (2026-10-30)

Question asked: can those records be backfilled? Answered from the records themselves.

| Tier | Count | `rejected_attempt` | Chosen body | Verdict |
|:--|:--|:--|:--|:--|
| Silver | 5 | absent | `<filled>` | nothing stored the code; unrecoverable |
| Gold | 2 | present (125ch, 89ch) | `<filled>` | only the negative half survives; no pair can be rebuilt |

All seven scope signatures still resolve to existing fixtures (`/tmp/fill_probe`,
`/tmp/gb199_probe`, `/tmp/gb-gold-probe`, `/tmp/gb-gold-crate`), so *regeneration* is possible -
resurrection is not. The content hash was computed over the placeholder trace, so it cannot be used
to find a better copy either.

Decision: **clean base, no backfill.** Four reasons.
1. Unrecoverable as pairs - the chosen side was never written anywhere.
2. Regeneration produces better records: the current path has five fewer defects than when these
   were written, so a re-run is more likely to yield a usable pair than the original attempt did.
3. A gate that always prints seven known rejections stops being read - the noise blindness is the
   real cost, not the seven rows.
4. The fixtures still exist, so any regeneration that matters can be done deliberately per fixture.

Executed non-destructively: the seven moved to `.gaugeblock/spool/quarantine/`
(`outbox-placeholders-2026-10-30.jsonl`, sha256 prefix `9fce3077eeffceb0`), outbox left holding 34
records with zero placeholders. Re-ingest: **34 accepted, 0 rejected**, shards SFT 17 / DPO 1.
The spool directory is gitignored, so this document and the ledger carry the evidence.

## Cross-fixture yield study, 2026-10-30 (supersedes the 4-run single-fixture figure)

Three fixtures, three fills each, run sequentially. Each fixture has a documented helper in a
sibling module and a `todo!()` body that should delegate to it.

| # | Fixture | Hole | Tier | Attempts | Body |
|:--|:--|:--|:--|:--|:--|
| a1 | gb-gold-crate | grapheme_count | **Gold** | 2 | 258 |
| a2 | gb-gold-crate | grapheme_count | Silver | 1 | 225 |
| a3 | gb-gold-crate | grapheme_count | Silver | 1 | 255 |
| b1 | gb-fix-net | parse_port | **Gold** | 2 | 278 |
| b2 | gb-fix-net | parse_port | **Gold** | 2 | 95 |
| b3 | gb-fix-net | parse_port | Silver | 1 | 283 |
| c1 | gb-fix-stats | sum_positive | Silver | 1 | 256 |
| c2 | gb-fix-stats | sum_positive | Silver | 1 | 256 |
| c3 | gb-fix-stats | sum_positive | Silver | 1 | 256 |

**9/9 produced a validated candidate. 3/9 Gold (a repair, therefore a DPO pair). Zero failures.**
Every body was real code - no placeholder anywhere.

Per fixture: `parse_port` 2/3 Gold, `grapheme_count` 1/3, `sum_positive` 0/3. The spread is
informative rather than noise: `sum_positive` is a one-liner delegation that compiles on the first
try, so it cannot produce a repair; `parse_port` needs range and error handling that the first
attempt plausibly gets wrong, which is what makes a repair likely. **Gold rate is a property of the
hole, not of the pipeline** - a corpus of trivial holes will be all-Silver no matter how good the
loop is, and a corpus of Gold pairs needs holes with real failure modes.

Corpus effect of the study: spool 43 records (39 Silver, 4 Gold), ingest 43/43 accepted with zero
rejections, shards SFT 26 rows and **DPO 4 rows**, each carrying a real body (245, 258, 278 and 95
characters).

Caveat, stated because it matters: three fixtures and nine runs is a usable signal, not a
population. The 33% figure should be read as "one in three fills repairs", not as a rate that will
hold on unfamiliar code.
