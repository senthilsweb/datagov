# Spec delta — `dataset-profiling` (add-datagov-core-cli)

### Requirement: Column statistics

`datagov profile` SHALL compute, per applicable column: row count, null
count and percentage, distinct count, uniqueness percentage, min, max,
mean, median, standard deviation, quantiles, string-length statistics,
top values, frequency distribution, inferred semantic type, and
possible-identifier flags (PRD §10.2).

#### Scenario: Profile selected columns with sampling

**When** the user runs
`datagov profile customers.parquet --columns email,state --sample 10000`
**Then** only the named columns are profiled
**And** the envelope records the sample size used.

### Requirement: Deterministic output

Profiling the same input with the same flags SHALL produce identical
results (envelope `run` block excepted).

#### Scenario: Two consecutive runs

**Given** an unchanged input file
**When** `datagov profile customers.csv --output json` runs twice
**Then** the `profile` sections of both envelopes are byte-identical.

### Requirement: Top values are masked when sensitive

Top-value and frequency listings SHALL pass through the shared masking
layer when the column is flagged as a possible identifier or matches a
PII recognizer.

#### Scenario: Top values of an SSN-like column

**Given** a column whose values match the US_SSN recognizer
**When** its top values appear in profile output
**Then** each listed value is masked.
