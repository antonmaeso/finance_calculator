# 20-year scenarios: sell vs keep & let

All runs use your inputs (horizon set to **20 years**, "count selling costs & CGT" on), changing one thing at a time.
Net worth = property values − mortgages + cash, with the first home valued as if sold at year 20 (agent fees, legal and CGT taken off).
Each input file is in [`inputs/`](inputs/). Open one in the app with `HOUSE_CALC_INPUTS=docs/scenarios_20y/inputs/<file>.json cargo run --release`.

| # | Scenario | Sell @20y | Keep & let @20y | Keep − Sell | Better |
|---|---|---:|---:|---:|---|
| 1 | **Baseline** (your inputs) | £1,333,600 | £1,333,935 | **+£335** | dead heat |
| 3 | Release £100k equity (not £70k) | £1,333,600 | £1,337,429 | +£3,829 | ≈ tie |
| 4 | Release only £40k (95% LTV on new home) | £1,333,600 | £1,330,566 | −£3,034 | ≈ tie |
| 5 | Let, then sell after 2.5 yrs (ADS refunded) | £1,333,600 | £1,266,761 | −£66,839 | Sell * |
| 6 | BTL on repayment, not interest-only | £1,333,600 | £1,338,377 | +£4,777 | ≈ tie |
| 7 | Rent £1,150/mo (not £1,350) | £1,333,600 | £1,284,749 | −£48,851 | **Sell** |
| 8 | Rent £1,550/mo | £1,333,600 | £1,381,869 | +£48,269 | **Keep** |
| 9 | Lower rates (4.5% mortgage, 5% BTL, 3.5% savings) | £1,249,698 | £1,352,447 | +£102,749 | **Keep** |
| 10 | Flat-ish prices (1% / 1.5%, rent +2%) | £1,195,823 | £1,090,233 | −£105,589 | **Sell** |
| 11 | Hot prices (4% both homes) | £1,449,103 | £1,557,169 | +£108,066 | **Keep** |
| 12 | Savings only earn 3% | £1,200,805 | £1,242,725 | +£41,920 | **Keep** |
| 13 | Self-manage the let (no 10% agent fee) | £1,333,600 | £1,373,439 | +£39,839 | **Keep** |
| 14 | Cheaper new home (£260k) | £1,374,194 | £1,380,988 | +£6,794 | ≈ tie |
| 2 | Keep without remortgaging (no equity release) | — | — | — | **Not feasible**: £14.1k short on completion day, 100% LTV, cash runs out in year 1 |

\* The model keeps the sale proceeds in savings (5%, taxed) while the 6% new mortgage keeps running. If you used the proceeds to pay down the new mortgage, this scenario would come out roughly level with selling now.

## Break-even points (baseline)

| What has to be true for keeping to beat selling | Value |
|---|---|
| Rent | ≥ £1,349/mo (you assume £1,350) |
| First home price growth | ≥ 2.49%/yr (you assume 2.5%) |
| Savings rate (above this, selling wins) | 5.02% (you assume 5.0%) |
| Rent that covers BTL mortgage + costs + tax (cash-flow neutral) | £609/mo |

Year-one letting: 6.75% gross yield, 4.95% net, ICR 343% (passes easily), about +£453/mo net cash after tax.

## Screenshots

Baseline, every tab:

| Overview | Mortgages | Letting |
|---|---|---|
| ![](s1_baseline_tab0.png) | ![](s1_baseline_tab1.png) | ![](s1_baseline_tab2.png) |

| Taxes | Sell & save | What-if explorer |
|---|---|---|
| ![](s1_baseline_tab3.png) | ![](s1_baseline_tab4.png) | ![](s1_baseline_tab5.png) |

Scenario overviews:

| | |
|---|---|
| 3 Release £100k ![](s3_release_100k.png) | 4 Release £40k ![](s4_release_40k.png) |
| 5 Let then sell at 2.5y ![](s5_let_then_sell_2_5y.png) | 6 BTL repayment ![](s6_btl_repayment.png) |
| 7 Rent £1,150 ![](s7_rent_1150.png) | 8 Rent £1,550 ![](s8_rent_1550.png) |
| 9 Lower rates ![](s9_low_rates.png) | 10 Flat prices ![](s10_flat_house_prices.png) |
| 11 Hot prices ![](s11_hot_house_prices.png) | 12 Savings 3% ![](s12_savings_3pct.png) |
| 13 Self-managed ![](s13_self_managed.png) | 14 New home £260k ![](s14_cheaper_new_home_260k.png) |
| 2 No equity release (infeasible) ![](s2_no_equity_release.png) | |

> A model to help you think, not financial or tax advice.
