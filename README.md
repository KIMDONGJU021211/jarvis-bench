# jarvis-bench

**A benchmark log of a local-model agent researching live Korean websites and turning what it read into files — losses included.**

[한국어](README.ko.md) · Method: [docs/METHOD.ko.md](docs/METHOD.ko.md) · Results: [docs/RESULTS.ko.md](docs/RESULTS.ko.md) · When our own instruments lied: [docs/MISMEASUREMENTS.ko.md](docs/MISMEASUREMENTS.ko.md)

The detailed documents are in Korean. This page is a summary.

## What was measured

JARVIS is a local-first agent we are building. Each run is one request like *"Find three accounting job postings in Yeosu on Saramin and put company, employment type, deadline and posting URL in a table document."*
It runs on real sites (Wishket, YES24, Saramin, Danawa, Musinsa, …) with a local model (Gemma 4 26B QAT on an RTX 3090), and the produced file is graded **by code**:

- **chain** — did one run both read pages and write a file
- **table fill** — of the rows × columns that were asked for, how many cells hold a value (placeholders like "unverifiable" do not count)
- **form & grounding** — title, summary, leaked markup, header row, placeholder padding, sources, and whether every number and URL in the file appears in pages the run actually read

The agent itself is not in this repository.

## Headline

| | 5 tasks we iterated on (2026-09-13, n=3) | 5 held-out tasks committed before seeing results (2026-09-14, n=1) |
|---|---|---|
| file produced | 14/15 | 4/5 |
| table fill | 76% | 55% |
| form & grounding all pass | 9/14 | 2/4 |
| numbers not found in read pages | 0 | 0 |

The iterated score carried a familiar-site effect. Fixes to the document pipeline generalised to unseen tasks; finding fields on unfamiliar detail pages did not.
One earlier run has 4 numbers we could not check because that run errored and stored no evidence — see the results page.

## The most important part

We logged more cases of the **score being wrong** than of the score going up. The run that started at 88% was n=2; rerunning its one failure showed a task that failed three times out of four.
Most defects found were in our wiring and our graders, not in the model.

## Contents

```
results/   raw runs (_runs.json) and grades (_grade.json), named date_samplesize_build
bench/     multiturn_bench.py · content_grade.py · doc_form.py
docs/      method · results · mismeasurements (Korean)
msix/      what a full-trust desktop agent can still do inside MSIX (file virtualization, child processes, loopback) — probe + measured table
```

2026-09-15 additions: a usage-limit takeover run on a fresh profile, **including the step that failed** (a "list remaining tasks only" handoff that ran 239.5 s and wrote a file), and six more entries in the mismeasurement log — one of them a shell that was measuring AppData from inside another app's MSIX sandbox.

The runner talks to a local JARVIS instance, so runs cannot be reproduced without it. The graders only need files. Number normalisation comes from [evidence-ledger](https://github.com/KIMDONGJU021211/evidence-ledger).

Files produced by the agent are not included (they contain text copied from the sites). Local paths, accounts and tokens were machine-scanned out before publishing.

## Running the graders

```bash
pip install git+https://github.com/KIMDONGJU021211/evidence-ledger python-docx openpyxl
python bench/content_grade.py <runs.json>
```

## License

Apache-2.0. See [AUTHORS](AUTHORS).
