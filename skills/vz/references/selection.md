# vz chart selection

How `vz` chooses a chart from column types and the hints you pass. Load this
when the auto-selected chart is wrong or you need to predict it.

## Contents

- Explicit axis pairs
- One axis hinted
- No hints
- Color and bar behavior

## Explicit axis pairs

With both axes given, the chart comes from the resolved type pair; reversed
pairs are normalized (`-x revenue -y date` renders `x=date`, a Line).

| X type | Y type | Chart |
|---|---|---|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| Quantitative | Temporal | Line (axes flipped) |
| Quantitative | Categorical | Bar (axes flipped) |
| Categorical | Categorical | Heatmap |
| anything else (Nominal, Temporal × Temporal) | — | Bar (fallback + warning) |

The fallback and its `warning: no chart rule for ...` fire **only** for an
explicit resolved pair with no dedicated rule. Auto-selection never falls back.

## One axis hinted

- **Only Y:** first Temporal → Line when Y is Quantitative (else Bar + fallback
  warning); else first Categorical → Bar when Y is Quantitative, else Heatmap
  `x=y=Y`; else another Quantitative → Scatter; else lone Y (Quantitative or
  Nominal) → Histogram.
- **Only X:** first other Quantitative → pair chart; else categorical X → Bar of
  row counts (`y=count(x)`); else quantitative X → Histogram; else error.

## No hints

Temporal × Quantitative → Line; Categorical × Quantitative → Bar; ≥ 2
Quantitative → Scatter; 1 Quantitative → Histogram; ≥ 2 Categorical → Heatmap;
otherwise the no-chart error listing the detected columns. Nominal columns do
not auto-produce a Bar.

## Color and bar behavior

Auto color is the first categorical column not used as X/Y, applied on most
hinted/explicit paths (Line/Scatter split series; on Bar it feeds the legend
only). Some paths leave it unset — for example only-Y paired with a Quantitative
X, and the no-hint Categorical × Quantitative Bar. Extra `-y` columns suppress
auto color.

Bar charts ignore `-c` for data: grouped/stacked bars do not exist, so heights
stay aggregated over all rows and the color column only reaches the summary
legend. An explicit `-c` warns; an auto-detected color does not.

Override any choice with `-t line|bar|scatter|histogram|heatmap`.
