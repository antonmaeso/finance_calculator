//! All user-adjustable inputs. Rates are stored in percent so they map
//! directly onto UI sliders.

use serde::{Deserialize, Serialize};

use crate::tax::TaxRules;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepositMode {
    /// Put a fixed amount down.
    Fixed,
    /// Put down everything available except the cash buffer.
    MaxAvailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    /// Gross annual non-property income (salary, pension…).
    pub income: f64,
    /// Share of the first property (and its rent/gains) in percent.
    pub share_pct: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentHome {
    pub value: f64,
    pub mortgage_balance: f64,
    pub rate_pct: f64,
    pub fixed_years_left: f64,
    pub revert_rate_pct: f64,
    pub term_years_left: f64,
    pub growth_pct: f64,

    /// What you originally paid, plus purchase costs and improvements (CGT base cost).
    pub purchase_price: f64,
    pub purchase_costs: f64,
    pub years_owned: f64,
    /// Years you lived in it as your main residence (for private residence relief).
    pub years_lived_in: f64,

    /// Estate agent fee in percent of sale price.
    pub selling_agent_pct: f64,
    /// Solicitor, Home Report, marketing…
    pub selling_fixed_costs: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewHome {
    pub price: f64,
    pub growth_pct: f64,
    pub first_time_buyer: bool,
    /// Solicitor, survey, lender fees, removals.
    pub purchase_fees: f64,

    pub rate_pct: f64,
    pub fixed_years: f64,
    pub revert_rate_pct: f64,
    pub term_years: f64,
    pub overpayment_monthly: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Letting {
    /// Remortgage the first home onto a buy-to-let mortgage. If false, the
    /// existing residential mortgage continues (needs lender consent to let).
    pub remortgage_to_btl: bool,
    /// Extra cash borrowed against the first home on remortgage.
    pub release_equity: f64,
    pub btl_rate_pct: f64,
    pub btl_fixed_years: f64,
    pub btl_revert_rate_pct: f64,
    pub btl_term_years: f64,
    pub btl_interest_only: bool,
    pub remortgage_fees: f64,
    /// Lender stress test inputs (shown as a check, not enforced).
    pub lender_stress_rate_pct: f64,
    pub lender_icr_pct: f64,

    pub monthly_rent: f64,
    pub rent_growth_pct: f64,
    pub void_weeks_per_year: f64,
    pub letting_agent_pct: f64,
    pub maintenance_pct_of_rent: f64,
    /// Landlord insurance, gas/electrical safety, EPC, landlord registration, etc.
    pub fixed_costs_annual: f64,
    /// One-off costs to get the property ready to let.
    pub setup_costs: f64,

    /// Sell the first home at some point during the horizon.
    pub plan_sale: bool,
    pub sale_after_years: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Savings {
    /// Cash you have today, before any move.
    pub cash: f64,
    /// Cash you want to keep aside, never used as deposit.
    pub cash_buffer: f64,
    /// Interest you can earn on spare cash (AER, percent).
    pub rate_pct: f64,
    /// Share of the interest that's sheltered (ISA), percent.
    pub isa_share_pct: f64,
    /// Money from salary each month that goes to housing (mortgages, letting
    /// shortfalls) with anything left over saved. Same in both strategies.
    pub monthly_budget: f64,
    /// Annual increase of the monthly budget, percent.
    pub budget_growth_pct: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inputs {
    pub owners: Vec<Owner>,
    pub income_growth_pct: f64,
    pub current: CurrentHome,
    pub new_home: NewHome,
    pub letting: Letting,
    pub savings: Savings,

    pub sell_deposit_mode: DepositMode,
    pub sell_deposit: f64,
    pub keep_deposit_mode: DepositMode,
    pub keep_deposit: f64,

    pub horizon_years: u32,
    /// Deduct selling costs and CGT from the first home in the final net worth
    /// (i.e. compare as if it were sold at the end of the horizon).
    pub liquidate_at_end: bool,

    pub tax: TaxRules,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            owners: vec![
                Owner { name: "You".into(), income: 55_000.0, share_pct: 50.0 },
                Owner { name: "Partner".into(), income: 40_000.0, share_pct: 50.0 },
            ],
            income_growth_pct: 2.5,
            current: CurrentHome {
                value: 240_000.0,
                mortgage_balance: 130_000.0,
                rate_pct: 4.2,
                fixed_years_left: 2.0,
                revert_rate_pct: 4.5,
                term_years_left: 22.0,
                growth_pct: 3.0,
                purchase_price: 180_000.0,
                purchase_costs: 3_500.0,
                years_owned: 6.0,
                years_lived_in: 6.0,
                selling_agent_pct: 1.2,
                selling_fixed_costs: 3_000.0,
            },
            new_home: NewHome {
                price: 400_000.0,
                growth_pct: 3.0,
                first_time_buyer: false,
                purchase_fees: 3_500.0,
                rate_pct: 4.3,
                fixed_years: 5.0,
                revert_rate_pct: 4.5,
                term_years: 30.0,
                overpayment_monthly: 0.0,
            },
            letting: Letting {
                remortgage_to_btl: true,
                release_equity: 40_000.0,
                btl_rate_pct: 4.9,
                btl_fixed_years: 5.0,
                btl_revert_rate_pct: 5.5,
                btl_term_years: 25.0,
                btl_interest_only: true,
                remortgage_fees: 2_000.0,
                lender_stress_rate_pct: 5.5,
                lender_icr_pct: 125.0,
                monthly_rent: 1_150.0,
                rent_growth_pct: 3.0,
                void_weeks_per_year: 2.0,
                letting_agent_pct: 10.0,
                maintenance_pct_of_rent: 8.0,
                fixed_costs_annual: 900.0,
                setup_costs: 2_000.0,
                plan_sale: false,
                sale_after_years: 2.5,
            },
            savings: Savings {
                cash: 60_000.0,
                cash_buffer: 10_000.0,
                rate_pct: 4.0,
                isa_share_pct: 30.0,
                monthly_budget: 2_000.0,
                budget_growth_pct: 2.5,
            },
            sell_deposit_mode: DepositMode::MaxAvailable,
            sell_deposit: 100_000.0,
            keep_deposit_mode: DepositMode::MaxAvailable,
            keep_deposit: 60_000.0,
            horizon_years: 15,
            liquidate_at_end: true,
            tax: TaxRules::default(),
        }
    }
}

impl Inputs {
    /// Ownership shares normalised to sum to 1.
    pub fn shares(&self) -> Vec<f64> {
        let total: f64 = self.owners.iter().map(|o| o.share_pct.max(0.0)).sum();
        if total <= 0.0 {
            let n = self.owners.len().max(1) as f64;
            return self.owners.iter().map(|_| 1.0 / n).collect();
        }
        self.owners.iter().map(|o| o.share_pct.max(0.0) / total).collect()
    }

    pub fn selling_costs(&self, price: f64) -> f64 {
        price * self.current.selling_agent_pct / 100.0 + self.current.selling_fixed_costs
    }
}
