//! Break-even solvers and sensitivity sweeps built on top of the simulation.

use serde::{Deserialize, Serialize};

use crate::model::Inputs;
use crate::scenario::{letting_year_one, Comparison};

/// An input you can sweep across a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Param {
    FirstHomeGrowth,
    NewHomeGrowth,
    MonthlyRent,
    RentGrowth,
    SavingsRate,
    NewMortgageRate,
    BtlRate,
    ReleaseEquity,
    NewHomePrice,
    SaleYear,
}

impl Param {
    pub const ALL: [Param; 10] = [
        Param::FirstHomeGrowth,
        Param::NewHomeGrowth,
        Param::MonthlyRent,
        Param::RentGrowth,
        Param::SavingsRate,
        Param::NewMortgageRate,
        Param::BtlRate,
        Param::ReleaseEquity,
        Param::NewHomePrice,
        Param::SaleYear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Param::FirstHomeGrowth => "First home price growth (%/yr)",
            Param::NewHomeGrowth => "New home price growth (%/yr)",
            Param::MonthlyRent => "Monthly rent (£)",
            Param::RentGrowth => "Rent growth (%/yr)",
            Param::SavingsRate => "Savings interest rate (%)",
            Param::NewMortgageRate => "New mortgage rate (%)",
            Param::BtlRate => "Buy-to-let rate (%)",
            Param::ReleaseEquity => "Equity released (£)",
            Param::NewHomePrice => "New home price (£)",
            Param::SaleYear => "Sell first home after (years)",
        }
    }

    pub fn get(self, i: &Inputs) -> f64 {
        match self {
            Param::FirstHomeGrowth => i.current.growth_pct,
            Param::NewHomeGrowth => i.new_home.growth_pct,
            Param::MonthlyRent => i.letting.monthly_rent,
            Param::RentGrowth => i.letting.rent_growth_pct,
            Param::SavingsRate => i.savings.rate_pct,
            Param::NewMortgageRate => i.new_home.rate_pct,
            Param::BtlRate => i.letting.btl_rate_pct,
            Param::ReleaseEquity => i.letting.release_equity,
            Param::NewHomePrice => i.new_home.price,
            Param::SaleYear => i.letting.sale_after_years,
        }
    }

    pub fn set(self, i: &mut Inputs, v: f64) {
        match self {
            Param::FirstHomeGrowth => i.current.growth_pct = v,
            Param::NewHomeGrowth => i.new_home.growth_pct = v,
            Param::MonthlyRent => i.letting.monthly_rent = v,
            Param::RentGrowth => i.letting.rent_growth_pct = v,
            Param::SavingsRate => i.savings.rate_pct = v,
            // Assume the follow-on rate moves with the initial one.
            Param::NewMortgageRate => {
                let gap = i.new_home.revert_rate_pct - i.new_home.rate_pct;
                i.new_home.rate_pct = v;
                i.new_home.revert_rate_pct = v + gap;
            }
            Param::BtlRate => {
                let gap = i.letting.btl_revert_rate_pct - i.letting.btl_rate_pct;
                i.letting.btl_rate_pct = v;
                i.letting.btl_revert_rate_pct = v + gap;
            }
            Param::ReleaseEquity => i.letting.release_equity = v,
            Param::NewHomePrice => i.new_home.price = v,
            Param::SaleYear => {
                i.letting.plan_sale = true;
                i.letting.sale_after_years = v;
            }
        }
    }

    /// A sensible default range around the current inputs.
    pub fn default_range(self, i: &Inputs) -> (f64, f64) {
        match self {
            Param::FirstHomeGrowth | Param::NewHomeGrowth | Param::RentGrowth => (-3.0, 8.0),
            Param::MonthlyRent => {
                let r = i.letting.monthly_rent.max(200.0);
                (r * 0.5, r * 1.6)
            }
            Param::SavingsRate => (0.0, 8.0),
            Param::NewMortgageRate | Param::BtlRate => (1.0, 9.0),
            Param::ReleaseEquity => {
                let max_release = (i.current.value * 0.75 - i.current.mortgage_balance).max(0.0);
                (0.0, max_release.max(1_000.0))
            }
            Param::NewHomePrice => {
                let p = i.new_home.price.max(50_000.0);
                (p * 0.6, p * 1.5)
            }
            Param::SaleYear => (0.0, i.horizon_years as f64),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SweepPoint {
    pub x: f64,
    pub sell: f64,
    pub keep: f64,
    pub advantage: f64,
}

pub fn sweep(inputs: &Inputs, p: Param, lo: f64, hi: f64, steps: usize, liquid: bool) -> Vec<SweepPoint> {
    let steps = steps.max(2);
    (0..steps)
        .map(|k| {
            let x = lo + (hi - lo) * k as f64 / (steps - 1) as f64;
            let mut i = inputs.clone();
            p.set(&mut i, x);
            let c = Comparison::run(&i);
            let (sell, keep) = (c.sell.final_net_worth(liquid), c.keep.final_net_worth(liquid));
            SweepPoint { x, sell, keep, advantage: keep - sell }
        })
        .collect()
}

/// Grid of keep-minus-sell advantage. Row-major, `ny` rows of `nx` values,
/// row 0 = lowest y.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grid {
    pub px: Option<Param>,
    pub py: Option<Param>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub nx: usize,
    pub ny: usize,
    pub values: Vec<f64>,
}

pub fn grid(inputs: &Inputs, px: Param, x_range: (f64, f64), py: Param, y_range: (f64, f64), nx: usize, ny: usize, liquid: bool) -> Grid {
    let (nx, ny) = (nx.max(2), ny.max(2));
    let mut values = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        let y = y_range.0 + (y_range.1 - y_range.0) * j as f64 / (ny - 1) as f64;
        for k in 0..nx {
            let x = x_range.0 + (x_range.1 - x_range.0) * k as f64 / (nx - 1) as f64;
            let mut i = inputs.clone();
            px.set(&mut i, x);
            py.set(&mut i, y);
            values.push(Comparison::run(&i).advantage(liquid));
        }
    }
    Grid { px: Some(px), py: Some(py), x_range, y_range, nx, ny, values }
}

/// Find x in [lo, hi] where f(x) = 0 by bisection. Returns None when f has
/// the same sign at both ends.
pub fn bisect(mut f: impl FnMut(f64) -> f64, mut lo: f64, mut hi: f64) -> Option<f64> {
    let mut flo = f(lo);
    let fhi = f(hi);
    if flo == 0.0 {
        return Some(lo);
    }
    if fhi == 0.0 {
        return Some(hi);
    }
    if flo.signum() == fhi.signum() {
        return None;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        let fm = f(mid);
        if fm.signum() == flo.signum() {
            lo = mid;
            flo = fm;
        } else {
            hi = mid;
        }
        if (hi - lo).abs() < 1e-6 {
            break;
        }
    }
    Some(0.5 * (lo + hi))
}

/// Key break-even numbers shown on the dashboard.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BreakEvens {
    /// Monthly rent at which year-one letting cash flow (after mortgage, costs and tax) is zero.
    pub rent_cashflow_neutral: Option<f64>,
    /// Monthly rent at which keeping and selling end level at the horizon.
    pub rent_vs_selling: Option<f64>,
    /// Annual growth of the first home needed for keeping to match selling.
    pub growth_vs_selling_pct: Option<f64>,
    /// Value the first home must reach by the horizon at that growth.
    pub required_value: Option<f64>,
    /// Savings rate at which selling and keeping end level.
    pub savings_rate_vs_keeping_pct: Option<f64>,
}

pub fn break_evens(inputs: &Inputs, liquid: bool) -> BreakEvens {
    let adv = |p: Param, v: f64| {
        let mut i = inputs.clone();
        p.set(&mut i, v);
        Comparison::run(&i).advantage(liquid)
    };
    let rent_cashflow_neutral = bisect(|r| letting_year_one(inputs, r).net_cashflow, 0.0, 50_000.0);
    let rent_vs_selling = bisect(|r| adv(Param::MonthlyRent, r), 0.0, 50_000.0);
    let growth = bisect(|g| adv(Param::FirstHomeGrowth, g), -20.0, 30.0);
    let required_value = growth.map(|g| inputs.current.value * (1.0 + g / 100.0).powi(inputs.horizon_years as i32));
    let savings = bisect(|r| adv(Param::SavingsRate, r), 0.0, 20.0);
    BreakEvens {
        rent_cashflow_neutral,
        rent_vs_selling,
        growth_vs_selling_pct: growth,
        required_value,
        savings_rate_vs_keeping_pct: savings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bisect_finds_root() {
        let r = bisect(|x| x * x - 2.0, 0.0, 2.0).unwrap();
        assert!((r - 2f64.sqrt()).abs() < 1e-5);
        assert!(bisect(|x| x + 10.0, 0.0, 1.0).is_none());
    }

    #[test]
    fn break_even_rent_zeroes_cashflow() {
        let i = Inputs::default();
        let b = break_evens(&i, true);
        let r = b.rent_cashflow_neutral.unwrap();
        assert!(letting_year_one(&i, r).net_cashflow.abs() < 1.0);
    }

    #[test]
    fn break_even_growth_levels_strategies() {
        let i = Inputs::default();
        if let Some(g) = break_evens(&i, true).growth_vs_selling_pct {
            let mut j = i.clone();
            j.current.growth_pct = g;
            assert!(Comparison::run(&j).advantage(true).abs() < 50.0);
        }
    }

    #[test]
    fn grid_has_expected_shape() {
        let i = Inputs::default();
        let g = grid(&i, Param::FirstHomeGrowth, (0.0, 5.0), Param::SavingsRate, (1.0, 5.0), 4, 3, true);
        assert_eq!(g.values.len(), 12);
    }
}
