# Task breakdown

The project was split into these tasks. ✅ = done in this version, ⬜ = ideas for next steps.

## 1. Foundations
- ✅ Cargo project, egui/eframe desktop window, egui_plot charts
- ✅ Library crate (`house_calc`) kept separate from the GUI so the maths is testable

## 2. Mortgage engine (`mortgage.rs`)
- ✅ Repayment and interest-only loans
- ✅ Initial fixed rate, then a follow-on rate
- ✅ Monthly overpayments
- ✅ Full amortisation schedule and total interest

## 3. Scottish tax engine (`tax.rs`)
- ✅ LBTT bands and first-time buyer relief
- ✅ ADS (8%) and the 36-month refund
- ✅ Scottish income tax 2026/27, personal allowance taper
- ✅ Rental profit tax with the Section 24 finance-cost credit and loss carry-forward
- ✅ UK savings interest tax (PSA, starting rate, ISA share)
- ✅ CGT with private residence relief (final 9 months), per owner, using their remaining basic-rate band
- ✅ All rates editable in the UI

## 4. Scenario simulation (`scenario.rs`)
- ✅ Completion-day cash flow: sale proceeds, equity release, fees, LBTT/ADS, deposit, LTV
- ✅ Month-by-month: mortgages, rent, voids, agent fees, maintenance, fixed costs, tax, savings interest
- ✅ Joint owners with ownership shares and separate incomes
- ✅ Optional later sale of the first home (CGT, ADS refund)
- ✅ Net worth, and net worth after selling costs/CGT, each year
- ✅ Warnings: cash shortfall on completion, budget not covering costs, LTV above 95%

## 5. Analysis (`analysis.rs`)
- ✅ Break-even rent (cash flow neutral, and vs selling)
- ✅ Break-even house price growth and the required future value
- ✅ Break-even savings rate
- ✅ Sweep any variable; 2-D heatmap of any two variables

## 6. User interface
- ✅ Sidebar with every input (sliders, plus typed values)
- ✅ Overview: verdict, KPIs, net worth chart, keep − sell chart, completion-day table
- ✅ Mortgages & interest, Letting, Taxes, Sell & save, What-if explorer, Other factors tabs
- ✅ Save / load named scenarios; export / import JSON
- ✅ Remembers your inputs between runs

## 7. Ideas for later
- ⬜ Rate scenarios over time (e.g. base rate path) instead of one follow-on rate
- ⬜ Stamp duty for buying in England (SDLT), for cross-border moves
- ⬜ Limited company buy-to-let (corporation tax, dividend extraction)
- ⬜ Monte-Carlo house price / rate paths with a probability-of-winning chart
- ⬜ Inflation-adjusted (real) values
- ⬜ PDF / PNG export of the charts
- ⬜ macOS `.app` bundle via `cargo bundle`
