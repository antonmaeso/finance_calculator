# Scottish house move calculator

A desktop app (Rust + [egui](https://github.com/emilk/egui)) for planning a move to a new house in Scotland. It compares the two main options side by side:

| Strategy | What happens |
|---|---|
| **Sell** | Sell the current home, put the equity towards the new one, save what's left. |
| **Keep & let** | Keep the current home (optionally remortgage to buy-to-let and release equity), rent it out, pay the Additional Dwelling Supplement (ADS) on the new home. You can also choose to sell it later; selling within 36 months gets the ADS refunded. |

Every input is a slider, and all charts update as you drag.

## Run it on your Mac

```bash
# 1. Install Rust (one-off; needs rustc 1.92 or newer — run `rustup update` if older)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. From this folder
cargo run --release
```

The first build takes a few minutes. After that it starts straight away. Your inputs and saved scenarios are remembered between runs (in `~/Library/Application Support/`).

For a text summary in the terminal: `cargo run --example summary` (or `cargo run --example summary my_plan.json` with a file exported from the app).

## What it calculates

| Question | Where |
|---|---|
| How much we owe the bank, and when | **Mortgages & interest**: balances, monthly payments, interest vs capital each year |
| How much interest we pay over time | **Mortgages & interest**: cumulative interest, whole-term cost, effect of overpayments |
| Taxes if we keep the first home and release equity | **Taxes**: LBTT + 8% ADS, Scottish income tax on rent (with the Section 24 20% credit), CGT on a later sale |
| Rent needed to cover the mortgage | **Letting**: break-even rent, cash flow at different rents, lender rent-cover (ICR) check |
| How much the first home must rise in value | **Overview** / **What-if explorer**: growth rate (and value) at which keeping matches selling |
| Taxes while renting it out | **Taxes** / **Letting**: tax each year per owner, marginal rate with the rent on top |
| If we sell, what saving at different rates earns | **Sell & save**: cash pot at 1–6% and the savings rate at which selling wins |
| Which conditions are most profitable | **What-if explorer**: sweep any variable, or map two at once on a heatmap |
| Other factors | **Other factors**: landlord registration, PRT tenancies, rent controls, EPC, the ADS refund deadline, 60-day CGT reporting… |

### How the comparison works

Both strategies start on the same completion day with the same savings. Each month, the same **monthly housing budget** from your salary goes into a cash pot, and housing costs (mortgages, letting costs, tax) come out of it. Rent goes in. Spare cash earns the savings rate, and that interest is taxed unless it's in an ISA. The comparison is **net worth after N years**: property values, minus mortgages, plus cash. With "Count selling costs & CGT" ticked, the first home is valued as if it were sold at the end.

### Tax rules (2026/27, editable in the sidebar)

- **LBTT**: 0% to £145k, 2% to £250k, 5% to £325k, 10% to £750k, 12% above. First-time buyer nil band £175k.
- **ADS**: 8% of the whole price, refundable if the previous main home is sold within 36 months.
- **Scottish income tax**: 19 / 20 / 21 / 42 / 45 / 48% with thresholds £16,537 / £29,526 / £43,662 / £75,000 / £125,140. The personal allowance tapers above £100k.
- **Rental profit**: taxed at Scottish rates. Mortgage interest gets a 20% tax credit (Section 24) instead of a deduction. Losses are carried forward.
- **Savings interest**: UK rates and bands, with a Personal Savings Allowance of £1,000 or £500 and the starting rate for savings.
- **CGT**: 18% / 24%, annual exempt amount £3,000. Private residence relief covers the years you lived there plus the final 9 months.

When a budget changes these, update them in **Tax rules** in the sidebar.

> This is a model to help you think. It is not financial or tax advice. Check the numbers with a mortgage adviser and a tax adviser before committing.

## Project layout

```
src/
  tax.rs        LBTT, ADS, Scottish income tax, savings tax, CGT, PRR
  mortgage.rs   repayment / interest-only loans, fixed-then-revert rates, overpayments
  model.rs      all inputs (+ sensible defaults)
  scenario.rs   month-by-month simulation of Sell vs Keep & let
  analysis.rs   break-even solvers, sweeps, 2-D grids
  app.rs        window layout, scenarios, save/load
  ui/           sidebar inputs, chart tabs, formatting helpers
examples/summary.rs   text report
docs/TASKS.md         the task breakdown / roadmap
```

`cargo test` runs the unit tests for the finance engine.
