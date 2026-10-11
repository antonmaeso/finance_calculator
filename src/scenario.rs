//! Month-by-month simulation of the two strategies:
//!
//! * **Sell** – sell the current home, use the equity towards the new one and
//!   save whatever is left.
//! * **Keep & let** – keep the current home (optionally remortgaging it to a
//!   buy-to-let and releasing equity), let it out, pay ADS on the new home.
//!
//! Every pound that isn't tied up in property lives in a single "cash pot"
//! that earns the savings rate. Each month the same housing budget from your
//! salary goes into the pot in both strategies, and housing cash flows
//! (mortgage payments, net rent, taxes) come out of it. Other income and
//! living costs are identical in both strategies so they're left out; a
//! negative pot means the budget doesn't cover that strategy.

use serde::{Deserialize, Serialize};

use crate::model::{DepositMode, Inputs};
use crate::mortgage::{Loan, LoanTerms};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Strategy {
    Sell,
    KeepAndLet,
}

impl Strategy {
    pub fn label(self) -> &'static str {
        match self {
            Strategy::Sell => "Sell first home",
            Strategy::KeepAndLet => "Keep & let first home",
        }
    }
}

/// Money in and out on completion day.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Upfront {
    pub starting_cash: f64,
    pub sale_price: f64,
    pub sale_costs: f64,
    pub mortgage_redeemed: f64,
    pub equity_released: f64,
    pub remortgage_fees: f64,
    pub letting_setup: f64,
    pub lbtt: f64,
    pub ads: f64,
    pub purchase_fees: f64,
    pub deposit: f64,
    pub new_mortgage: f64,
    pub ltv_pct: f64,
    pub cash_left: f64,
    pub btl_loan: f64,
}

impl Upfront {
    pub fn total_taxes_and_fees(&self) -> f64 {
        self.sale_costs + self.remortgage_fees + self.letting_setup + self.lbtt + self.ads + self.purchase_fees
    }
}

/// Snapshot at the end of each year plus the flows during that year.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct YearPoint {
    pub year: u32,
    pub first_value: f64,
    pub first_debt: f64,
    pub new_value: f64,
    pub new_debt: f64,
    pub cash: f64,
    /// Property values minus debts plus cash.
    pub net_worth: f64,
    /// Net worth after selling the first home (selling costs + CGT) — what you'd actually walk away with.
    pub net_worth_liquid: f64,

    pub new_interest: f64,
    pub new_principal: f64,
    pub btl_interest: f64,
    pub btl_principal: f64,
    pub rent_received: f64,
    pub letting_costs: f64,
    pub rental_tax: f64,
    pub savings_interest: f64,
    pub savings_tax: f64,
    pub cgt: f64,
    pub ads_refund: f64,
    /// Average monthly housing outgoings after rent and tax (positive = money out).
    pub monthly_net_cost: f64,

    pub cum_interest: f64,
    pub cum_tax: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScenarioResult {
    pub upfront: Upfront,
    pub years: Vec<YearPoint>,
    pub first_home_sold_year: Option<f64>,
    pub warnings: Vec<String>,
}

impl ScenarioResult {
    pub fn final_point(&self) -> &YearPoint {
        self.years.last().expect("at least year 0")
    }
    pub fn final_net_worth(&self, liquid: bool) -> f64 {
        let p = self.final_point();
        if liquid {
            p.net_worth_liquid
        } else {
            p.net_worth
        }
    }
    pub fn total_interest(&self) -> f64 {
        self.final_point().cum_interest
    }
    pub fn total_tax(&self) -> f64 {
        self.final_point().cum_tax + self.upfront.lbtt + self.upfront.ads
    }
}

/// Per-owner carry-forwards for rental income tax.
#[derive(Clone, Default)]
pub struct OwnerTaxState {
    pub loss_bf: f64,
    pub finance_bf: f64,
}

/// Rental income tax for one tax year, split across owners.
/// Returns (total tax, per-owner property profit).
pub fn rental_tax_for_year(
    inputs: &Inputs,
    year: u32,
    rent: f64,
    allowable_costs: f64,
    finance_costs: f64,
    state: &mut [OwnerTaxState],
) -> (f64, Vec<f64>) {
    let t = &inputs.tax;
    let growth = (1.0 + inputs.income_growth_pct / 100.0).powi(year as i32);
    let mut total = 0.0;
    let mut profits = Vec::new();
    for ((owner, share), st) in inputs.owners.iter().zip(inputs.shares()).zip(state.iter_mut()) {
        let income = owner.income * growth;
        let mut profit = share * (rent - allowable_costs);
        // Use brought-forward losses first.
        if profit > 0.0 {
            let used = st.loss_bf.min(profit);
            st.loss_bf -= used;
            profit -= used;
        } else {
            st.loss_bf += -profit;
            profit = 0.0;
        }
        let tax = t.scottish_income_tax(income + profit) - t.scottish_income_tax(income);
        // Section 24: finance costs give a 20% credit, limited to the property profit.
        let finance = share * finance_costs + st.finance_bf;
        let relieved = finance.min(profit);
        st.finance_bf = finance - relieved;
        let credit = (relieved * t.finance_cost_credit_rate).min(tax);
        total += tax - credit;
        profits.push(profit);
    }
    (total, profits)
}

/// CGT if the first home were sold for `price` at `years_from_now`.
pub fn first_home_cgt(inputs: &Inputs, price: f64, years_from_now: f64, extra_income: &[f64]) -> f64 {
    let c = &inputs.current;
    let gain = price - inputs.selling_costs(price) - c.purchase_price - c.purchase_costs;
    if gain <= 0.0 {
        return 0.0;
    }
    let owned = c.years_owned + years_from_now;
    let relief = inputs.tax.prr_fraction(c.years_lived_in, owned);
    let chargeable = gain * (1.0 - relief);
    let year = years_from_now.floor() as i32;
    let growth = (1.0 + inputs.income_growth_pct / 100.0).powi(year);
    inputs
        .owners
        .iter()
        .zip(inputs.shares())
        .enumerate()
        .map(|(i, (o, share))| {
            let income = o.income * growth + extra_income.get(i).copied().unwrap_or(0.0);
            inputs.tax.cgt(chargeable * share, income)
        })
        .sum()
}

fn new_loan_terms(inputs: &Inputs, principal: f64) -> LoanTerms {
    let n = &inputs.new_home;
    LoanTerms {
        principal,
        initial_rate_pct: n.rate_pct,
        fixed_years: n.fixed_years,
        revert_rate_pct: n.revert_rate_pct,
        term_years: n.term_years,
        interest_only: false,
        overpayment_monthly: n.overpayment_monthly,
    }
}

/// Terms of the loan secured on the first home while it is let.
pub fn first_home_loan_terms(inputs: &Inputs) -> LoanTerms {
    let c = &inputs.current;
    let l = &inputs.letting;
    if l.remortgage_to_btl {
        LoanTerms {
            principal: c.mortgage_balance + l.release_equity.max(0.0),
            initial_rate_pct: l.btl_rate_pct,
            fixed_years: l.btl_fixed_years,
            revert_rate_pct: l.btl_revert_rate_pct,
            term_years: l.btl_term_years,
            interest_only: l.btl_interest_only,
            overpayment_monthly: 0.0,
        }
    } else {
        LoanTerms {
            principal: c.mortgage_balance,
            initial_rate_pct: c.rate_pct,
            fixed_years: c.fixed_years_left,
            revert_rate_pct: c.revert_rate_pct,
            term_years: c.term_years_left,
            interest_only: false,
            overpayment_monthly: 0.0,
        }
    }
}

fn choose_deposit(mode: DepositMode, fixed: f64, available: f64, price: f64) -> f64 {
    let d = match mode {
        DepositMode::Fixed => fixed,
        DepositMode::MaxAvailable => available,
    };
    d.clamp(0.0, price)
}

pub fn simulate(inputs: &Inputs, strategy: Strategy) -> ScenarioResult {
    let t = &inputs.tax;
    let c = &inputs.current;
    let n = &inputs.new_home;
    let l = &inputs.letting;
    let s = &inputs.savings;
    let owners = inputs.owners.len().max(1);
    let months = inputs.horizon_years.max(1) * 12;

    let mut res = ScenarioResult::default();
    let mut up = Upfront { starting_cash: s.cash, ..Default::default() };
    let mut cash = s.cash;

    // ---------- Completion day ----------
    let mut first_loan: Option<Loan> = None;
    let mut first_value = c.value;
    let mut first_owned = true;
    let mut cum_tax = 0.0;

    match strategy {
        Strategy::Sell => {
            up.sale_price = c.value;
            up.sale_costs = inputs.selling_costs(c.value);
            up.mortgage_redeemed = c.mortgage_balance;
            cash += c.value - up.sale_costs - c.mortgage_balance;
            first_owned = false;
            first_value = 0.0;
            res.first_home_sold_year = Some(0.0);
            up.lbtt = t.lbtt(n.price, n.first_time_buyer);
        }
        Strategy::KeepAndLet => {
            let terms = first_home_loan_terms(inputs);
            if l.remortgage_to_btl {
                up.equity_released = l.release_equity.max(0.0);
                up.remortgage_fees = l.remortgage_fees;
                cash += up.equity_released - up.remortgage_fees;
            }
            up.btl_loan = terms.principal;
            up.letting_setup = l.setup_costs;
            cash -= l.setup_costs;
            first_loan = Some(Loan::new(terms));
            up.lbtt = t.lbtt(n.price, false);
            up.ads = t.ads(n.price);
        }
    }
    up.purchase_fees = n.purchase_fees;
    cash -= up.lbtt + up.ads + up.purchase_fees;

    let available = (cash - s.cash_buffer).max(0.0);
    let (mode, fixed) = match strategy {
        Strategy::Sell => (inputs.sell_deposit_mode, inputs.sell_deposit),
        Strategy::KeepAndLet => (inputs.keep_deposit_mode, inputs.keep_deposit),
    };
    up.deposit = choose_deposit(mode, fixed, available, n.price);
    cash -= up.deposit;
    up.new_mortgage = n.price - up.deposit;
    up.ltv_pct = if n.price > 0.0 { up.new_mortgage / n.price * 100.0 } else { 0.0 };
    up.cash_left = cash;

    if cash < 0.0 {
        res.warnings.push(format!(
            "{}: you are £{:.0} short on completion day — reduce the deposit or find more cash.",
            strategy.label(),
            -cash
        ));
    }
    if up.ltv_pct > 95.0 {
        res.warnings.push(format!("{}: new mortgage LTV {:.0}% is above what most lenders offer (95%).", strategy.label(), up.ltv_pct));
    }

    let mut new_loan = Loan::new(new_loan_terms(inputs, up.new_mortgage));
    let mut new_value = n.price;

    let g_first = (1.0 + c.growth_pct / 100.0).powf(1.0 / 12.0);
    let g_new = (1.0 + n.growth_pct / 100.0).powf(1.0 / 12.0);
    let g_rent = 1.0 + l.rent_growth_pct / 100.0;
    let save_r = s.rate_pct / 100.0 / 12.0;
    let occupancy = (1.0 - l.void_weeks_per_year / 52.0).clamp(0.0, 1.0);
    let sale_month = if strategy == Strategy::KeepAndLet && l.plan_sale {
        Some((l.sale_after_years * 12.0).round().max(0.0) as u32)
    } else {
        None
    };

    let mut tax_state = vec![OwnerTaxState::default(); owners];
    let mut cum_interest = 0.0;
    let mut neg_cash_warned = false;

    let snapshot = |year: u32,
                    first_owned: bool,
                    first_value: f64,
                    first_debt: f64,
                    new_value: f64,
                    new_debt: f64,
                    cash: f64,
                    profits: &[f64]| {
        let mut p = YearPoint {
            year,
            first_value: if first_owned { first_value } else { 0.0 },
            first_debt: if first_owned { first_debt } else { 0.0 },
            new_value,
            new_debt,
            cash,
            ..Default::default()
        };
        p.net_worth = p.first_value - p.first_debt + new_value - new_debt + cash;
        p.net_worth_liquid = if first_owned {
            let cgt = first_home_cgt(inputs, first_value, year as f64, profits);
            p.net_worth - inputs.selling_costs(first_value) - cgt
        } else {
            p.net_worth
        };
        p
    };

    let first_debt0 = first_loan.as_ref().map_or(0.0, |l| l.balance);
    res.years.push(snapshot(0, first_owned, first_value, first_debt0, new_value, new_loan.balance, cash, &[]));

    let mut yp = YearPoint::default();
    let mut year_rent = 0.0;
    let mut year_costs = 0.0;
    let mut year_finance = 0.0;
    let mut year_net_flow = 0.0;
    let mut monthly_rent = l.monthly_rent;
    let mut budget = s.monthly_budget;
    let mut last_profits = vec![0.0; owners];

    for m in 0..months {
        let year = m / 12;

        // --- Planned sale of the first home ---
        if first_owned && sale_month == Some(m) {
            let price = first_value;
            let costs = inputs.selling_costs(price);
            let debt = first_loan.as_mut().map_or(0.0, |l| l.settle());
            let cgt = first_home_cgt(inputs, price, m as f64 / 12.0, &last_profits);
            cash += price - costs - debt - cgt;
            yp.cgt += cgt;
            cum_tax += cgt;
            if m <= t.ads_refund_window_months {
                cash += up.ads;
                yp.ads_refund += up.ads;
            }
            first_owned = false;
            first_loan = None;
            res.first_home_sold_year = Some(m as f64 / 12.0);
        }

        // --- New home mortgage ---
        let nm = new_loan.step();
        yp.new_interest += nm.interest;
        yp.new_principal += nm.principal;
        cum_interest += nm.interest;
        let mut flow = -nm.payment;

        // --- Letting the first home ---
        if first_owned {
            if let Some(loan) = first_loan.as_mut() {
                let bm = loan.step();
                yp.btl_interest += bm.interest;
                yp.btl_principal += bm.principal;
                cum_interest += bm.interest;
                year_finance += bm.interest;
                flow -= bm.payment;
            }
            let rent = monthly_rent * occupancy;
            let costs = rent * (l.letting_agent_pct + l.maintenance_pct_of_rent) / 100.0 + l.fixed_costs_annual / 12.0;
            year_rent += rent;
            year_costs += costs;
            yp.rent_received += rent;
            yp.letting_costs += costs;
            flow += rent - costs;
        }

        // --- Cash pot interest (negative pot = borrowing at the same rate) ---
        let interest = cash * save_r;
        yp.savings_interest += interest;
        cash += interest + flow + budget;
        year_net_flow += flow;

        first_value *= g_first;
        new_value *= g_new;

        if cash < -1.0 && !neg_cash_warned {
            neg_cash_warned = true;
            res.warnings.push(format!(
                "{}: your monthly budget doesn't cover the costs — cash runs out in year {}.",
                strategy.label(),
                year + 1
            ));
        }

        // --- Year end: taxes and snapshot ---
        if m % 12 == 11 {
            let (rtax, profits) = rental_tax_for_year(inputs, year, year_rent, year_costs, year_finance, &mut tax_state);
            yp.rental_tax = rtax;
            last_profits = profits;

            // Savings interest tax, split between owners.
            let growth = (1.0 + inputs.income_growth_pct / 100.0).powi(year as i32);
            let taxable_interest = yp.savings_interest.max(0.0) * (1.0 - s.isa_share_pct / 100.0);
            yp.savings_tax = inputs
                .owners
                .iter()
                .zip(inputs.shares())
                .zip(last_profits.iter())
                .map(|((o, share), profit)| t.savings_tax(o.income * growth + profit, taxable_interest * share))
                .sum();

            cash -= yp.rental_tax + yp.savings_tax;
            cum_tax += yp.rental_tax + yp.savings_tax;
            year_net_flow -= yp.rental_tax;
            yp.monthly_net_cost = -year_net_flow / 12.0;

            let first_debt = first_loan.as_ref().map_or(0.0, |l| l.balance);
            let mut snap =
                snapshot(year + 1, first_owned, first_value, first_debt, new_value, new_loan.balance, cash, &last_profits);
            snap.new_interest = yp.new_interest;
            snap.new_principal = yp.new_principal;
            snap.btl_interest = yp.btl_interest;
            snap.btl_principal = yp.btl_principal;
            snap.rent_received = yp.rent_received;
            snap.letting_costs = yp.letting_costs;
            snap.rental_tax = yp.rental_tax;
            snap.savings_interest = yp.savings_interest;
            snap.savings_tax = yp.savings_tax;
            snap.cgt = yp.cgt;
            snap.ads_refund = yp.ads_refund;
            snap.monthly_net_cost = yp.monthly_net_cost;
            snap.cum_interest = cum_interest;
            snap.cum_tax = cum_tax;
            res.years.push(snap);

            yp = YearPoint::default();
            year_rent = 0.0;
            year_costs = 0.0;
            year_finance = 0.0;
            year_net_flow = 0.0;
            monthly_rent *= g_rent;
            budget *= 1.0 + s.budget_growth_pct / 100.0;
        }
    }

    res.upfront = up;
    res
}

/// Both strategies side by side.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Comparison {
    pub sell: ScenarioResult,
    pub keep: ScenarioResult,
}

impl Comparison {
    pub fn run(inputs: &Inputs) -> Self {
        Self { sell: simulate(inputs, Strategy::Sell), keep: simulate(inputs, Strategy::KeepAndLet) }
    }

    /// Keep-minus-sell net worth at the horizon. Positive = keeping wins.
    pub fn advantage(&self, liquid: bool) -> f64 {
        self.keep.final_net_worth(liquid) - self.sell.final_net_worth(liquid)
    }
}

/// Year-one letting figures, used for the break-even rent and lender checks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LettingYearOne {
    pub rent: f64,
    pub costs: f64,
    pub mortgage_payments: f64,
    pub mortgage_interest: f64,
    pub tax: f64,
    pub net_cashflow: f64,
    pub gross_yield_pct: f64,
    pub net_yield_pct: f64,
    pub icr_pct: f64,
    pub max_loan_by_icr: f64,
}

pub fn letting_year_one(inputs: &Inputs, monthly_rent: f64) -> LettingYearOne {
    let l = &inputs.letting;
    let occupancy = (1.0 - l.void_weeks_per_year / 52.0).clamp(0.0, 1.0);
    let rent = monthly_rent * 12.0 * occupancy;
    let costs = rent * (l.letting_agent_pct + l.maintenance_pct_of_rent) / 100.0 + l.fixed_costs_annual;
    let mut loan = Loan::new(first_home_loan_terms(inputs));
    let (mut pay, mut int) = (0.0, 0.0);
    for _ in 0..12 {
        let m = loan.step();
        pay += m.payment;
        int += m.interest;
    }
    let mut state = vec![OwnerTaxState::default(); inputs.owners.len().max(1)];
    let (tax, _) = rental_tax_for_year(inputs, 0, rent, costs, int, &mut state);
    let value = inputs.current.value.max(1.0);
    let stress = loan.terms.principal * l.lender_stress_rate_pct / 100.0;
    LettingYearOne {
        rent,
        costs,
        mortgage_payments: pay,
        mortgage_interest: int,
        tax,
        net_cashflow: rent - costs - pay - tax,
        gross_yield_pct: monthly_rent * 12.0 / value * 100.0,
        net_yield_pct: (rent - costs) / value * 100.0,
        icr_pct: if stress > 0.0 { monthly_rent * 12.0 / stress * 100.0 } else { f64::INFINITY },
        max_loan_by_icr: monthly_rent * 12.0 / (l.lender_icr_pct / 100.0) / (l.lender_stress_rate_pct / 100.0).max(1e-9),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sell_scenario_has_no_ads_and_keep_does() {
        let i = Inputs::default();
        let c = Comparison::run(&i);
        assert_eq!(c.sell.upfront.ads, 0.0);
        assert!((c.keep.upfront.ads - 32_000.0).abs() < 1.0);
        assert_eq!(c.sell.years.len(), i.horizon_years as usize + 1);
    }

    #[test]
    fn sell_scenario_cash_accounting() {
        let i = Inputs::default();
        let r = simulate(&i, Strategy::Sell);
        let u = &r.upfront;
        // Equity = 240k - 1.2% - 3k - 130k
        let equity = 240_000.0 - 2_880.0 - 3_000.0 - 130_000.0;
        let cash = 60_000.0 + equity - u.lbtt - u.purchase_fees;
        assert!((u.deposit - (cash - 10_000.0)).abs() < 1.0);
        assert!((u.cash_left - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn early_sale_refunds_ads() {
        let mut i = Inputs::default();
        i.letting.plan_sale = true;
        i.letting.sale_after_years = 1.0;
        let r = simulate(&i, Strategy::KeepAndLet);
        let refund: f64 = r.years.iter().map(|y| y.ads_refund).sum();
        assert!((refund - r.upfront.ads).abs() < 1.0);
        assert!(r.years.last().unwrap().first_value == 0.0);
    }

    #[test]
    fn zero_growth_zero_rates_conserves_money() {
        let mut i = Inputs::default();
        i.current.growth_pct = 0.0;
        i.new_home.growth_pct = 0.0;
        i.savings.rate_pct = 0.0;
        i.new_home.rate_pct = 0.0;
        i.new_home.revert_rate_pct = 0.0;
        i.savings.monthly_budget = 0.0;
        let r = simulate(&i, Strategy::Sell);
        let start = r.years[0].net_worth;
        let end = r.final_point().net_worth;
        // With no interest or growth, paying the mortgage moves cash into equity only.
        assert!((start - end).abs() < 1.0, "{start} vs {end}");
    }

    #[test]
    fn higher_rent_improves_keep() {
        let mut i = Inputs::default();
        let a = Comparison::run(&i).advantage(true);
        i.letting.monthly_rent += 300.0;
        let b = Comparison::run(&i).advantage(true);
        assert!(b > a);
    }
}
