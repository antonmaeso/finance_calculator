//! Scottish and UK tax rules relevant to moving house.
//!
//! All rates live in [`TaxRules`] so they can be edited from the UI when the
//! Scottish or UK budgets change. Defaults are the 2026/27 tax year.

use serde::{Deserialize, Serialize};

/// A marginal band: tax `rate` applies to the slice of the amount above
/// `threshold` (up to the next band's threshold).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub threshold: f64,
    pub rate: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaxRules {
    /// Label shown in the UI, e.g. "2026/27".
    pub year_label: String,

    // --- Land and Buildings Transaction Tax (Revenue Scotland) ---
    /// Residential LBTT bands on the purchase price.
    pub lbtt_bands: Vec<Band>,
    /// Nil-rate band when first-time buyer relief applies.
    pub lbtt_ftb_nil_band: f64,
    /// Additional Dwelling Supplement, flat rate on the whole price.
    pub ads_rate: f64,
    /// ADS only applies at or above this price.
    pub ads_min_price: f64,
    /// Months allowed to sell the previous main residence and reclaim ADS.
    pub ads_refund_window_months: u32,

    // --- Scottish income tax (non-savings, non-dividend income incl. rent) ---
    pub personal_allowance: f64,
    /// Personal allowance is reduced by £1 for every £2 above this.
    pub pa_taper_start: f64,
    /// Bands measured on *taxable* income (after personal allowance).
    pub scottish_bands: Vec<Band>,
    /// Section 24: finance costs relief given as a credit at this rate.
    pub finance_cost_credit_rate: f64,

    // --- UK rules used for savings interest and CGT ---
    /// UK basic-rate band width (taxable income), used for savings and CGT.
    pub uk_basic_band: f64,
    /// Total income above which the UK additional rate applies.
    pub uk_additional_threshold: f64,
    pub savings_basic_rate: f64,
    pub savings_higher_rate: f64,
    pub savings_additional_rate: f64,
    pub psa_basic: f64,
    pub psa_higher: f64,
    pub savings_starting_band: f64,

    // --- Capital Gains Tax on residential property ---
    pub cgt_annual_exempt: f64,
    pub cgt_basic_rate: f64,
    pub cgt_higher_rate: f64,
    /// Final period of ownership always covered by private residence relief.
    pub prr_final_period_months: f64,
}

impl Default for TaxRules {
    fn default() -> Self {
        Self {
            year_label: "2026/27".into(),
            lbtt_bands: vec![
                Band { threshold: 0.0, rate: 0.0 },
                Band { threshold: 145_000.0, rate: 0.02 },
                Band { threshold: 250_000.0, rate: 0.05 },
                Band { threshold: 325_000.0, rate: 0.10 },
                Band { threshold: 750_000.0, rate: 0.12 },
            ],
            lbtt_ftb_nil_band: 175_000.0,
            ads_rate: 0.08,
            ads_min_price: 40_000.0,
            ads_refund_window_months: 36,

            personal_allowance: 12_570.0,
            pa_taper_start: 100_000.0,
            // Thresholds on taxable income (gross minus the £12,570 allowance).
            scottish_bands: vec![
                Band { threshold: 0.0, rate: 0.19 },       // Starter   £12,571 – £16,537
                Band { threshold: 3_967.0, rate: 0.20 },   // Basic     £16,538 – £29,526
                Band { threshold: 16_956.0, rate: 0.21 },  // Intermediate £29,527 – £43,662
                Band { threshold: 31_092.0, rate: 0.42 },  // Higher    £43,663 – £75,000
                Band { threshold: 62_430.0, rate: 0.45 },  // Advanced  £75,001 – £125,140
                Band { threshold: 112_570.0, rate: 0.48 }, // Top       over £125,140
            ],
            finance_cost_credit_rate: 0.20,

            uk_basic_band: 37_700.0,
            uk_additional_threshold: 125_140.0,
            savings_basic_rate: 0.20,
            savings_higher_rate: 0.40,
            savings_additional_rate: 0.45,
            psa_basic: 1_000.0,
            psa_higher: 500.0,
            savings_starting_band: 5_000.0,

            cgt_annual_exempt: 3_000.0,
            cgt_basic_rate: 0.18,
            cgt_higher_rate: 0.24,
            prr_final_period_months: 9.0,
        }
    }
}

/// Apply marginal bands to `amount`.
pub fn banded_tax(amount: f64, bands: &[Band]) -> f64 {
    let mut tax = 0.0;
    for (i, band) in bands.iter().enumerate() {
        if amount <= band.threshold {
            break;
        }
        let upper = bands.get(i + 1).map_or(f64::INFINITY, |b| b.threshold);
        tax += (amount.min(upper) - band.threshold) * band.rate;
    }
    tax
}

impl TaxRules {
    /// LBTT on a residential purchase (excluding ADS).
    pub fn lbtt(&self, price: f64, first_time_buyer: bool) -> f64 {
        if !first_time_buyer {
            return banded_tax(price, &self.lbtt_bands);
        }
        // First-time buyer relief extends the nil-rate band.
        let mut bands = self.lbtt_bands.clone();
        if let Some(b) = bands.get_mut(1) {
            b.threshold = b.threshold.max(self.lbtt_ftb_nil_band);
        }
        banded_tax(price, &bands)
    }

    /// Additional Dwelling Supplement when buying while still owning another home.
    pub fn ads(&self, price: f64) -> f64 {
        if price >= self.ads_min_price {
            price * self.ads_rate
        } else {
            0.0
        }
    }

    pub fn personal_allowance_for(&self, income: f64) -> f64 {
        let excess = (income - self.pa_taper_start).max(0.0);
        (self.personal_allowance - excess / 2.0).max(0.0)
    }

    /// Scottish income tax on non-savings income (salary, pension, rental profit).
    pub fn scottish_income_tax(&self, income: f64) -> f64 {
        let taxable = (income - self.personal_allowance_for(income)).max(0.0);
        banded_tax(taxable, &self.scottish_bands)
    }

    /// Income tax on `savings_interest` for someone whose other income is `other_income`.
    /// Savings income uses UK (not Scottish) rates and bands.
    pub fn savings_tax(&self, other_income: f64, savings_interest: f64) -> f64 {
        if savings_interest <= 0.0 {
            return 0.0;
        }
        let total = other_income + savings_interest;
        let pa = self.personal_allowance_for(total);
        let basic_limit = pa + self.uk_basic_band;
        let psa = if total > self.uk_additional_threshold {
            0.0
        } else if total > basic_limit {
            self.psa_higher
        } else {
            self.psa_basic
        };

        // Interest sits on top of other income. Walk it through the bands.
        let mut remaining = savings_interest;
        let mut cursor = other_income;
        let mut tax = 0.0;

        // Personal allowance left over from other income.
        let pa_left = (pa - cursor).max(0.0).min(remaining);
        remaining -= pa_left;
        cursor += pa_left;

        // Starting rate for savings (0%) shrinks as non-savings income rises.
        let starting = (self.savings_starting_band - (other_income - pa).max(0.0)).max(0.0);
        let s = starting.min(remaining);
        remaining -= s;
        cursor += s;

        // Personal savings allowance (0%).
        let p = psa.min(remaining);
        remaining -= p;
        cursor += p;

        for (limit, rate) in [
            (basic_limit, self.savings_basic_rate),
            (self.uk_additional_threshold, self.savings_higher_rate),
            (f64::INFINITY, self.savings_additional_rate),
        ] {
            if remaining <= 0.0 {
                break;
            }
            let room = (limit - cursor).max(0.0).min(remaining);
            tax += room * rate;
            remaining -= room;
            cursor += room;
        }
        tax
    }

    /// CGT on `taxable_gain` (before the annual exemption) for one person
    /// with `income` of taxable income in the year of disposal.
    pub fn cgt(&self, gain: f64, income: f64) -> f64 {
        let taxable = (gain - self.cgt_annual_exempt).max(0.0);
        if taxable <= 0.0 {
            return 0.0;
        }
        let used = (income - self.personal_allowance_for(income)).max(0.0);
        let basic_left = (self.uk_basic_band - used).max(0.0);
        let at_basic = taxable.min(basic_left);
        at_basic * self.cgt_basic_rate + (taxable - at_basic) * self.cgt_higher_rate
    }

    /// Fraction of a gain covered by private residence relief.
    pub fn prr_fraction(&self, years_lived_in: f64, total_years_owned: f64) -> f64 {
        if total_years_owned <= 0.0 {
            return 1.0;
        }
        ((years_lived_in + self.prr_final_period_months / 12.0) / total_years_owned).clamp(0.0, 1.0)
    }

    /// Marginal Scottish rate at a given gross income (for display).
    pub fn scottish_marginal_rate(&self, income: f64) -> f64 {
        let d = 1.0;
        self.scottish_income_tax(income + d) - self.scottish_income_tax(income)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1.0, "{a} != {b}");
    }

    #[test]
    fn lbtt_examples() {
        let t = TaxRules::default();
        approx(t.lbtt(145_000.0, false), 0.0);
        // 2% of 105k + 5% of 50k = 2,100 + 2,500
        approx(t.lbtt(300_000.0, false), 4_600.0);
        // 2,100 + 3,750 + 10% of 75k
        approx(t.lbtt(400_000.0, false), 13_350.0);
        // FTB saves up to £600
        approx(t.lbtt(300_000.0, true), 4_000.0);
        approx(t.ads(300_000.0), 24_000.0);
    }

    #[test]
    fn scottish_income_tax_examples() {
        let t = TaxRules::default();
        approx(t.scottish_income_tax(12_570.0), 0.0);
        // Starter 3,967 * 19% + basic (30,000-16,537)… computed via bands
        let expected = 3_967.0 * 0.19 + 12_989.0 * 0.20 + (30_000.0 - 29_526.0) * 0.21;
        approx(t.scottish_income_tax(30_000.0), expected);
        assert!((t.scottish_marginal_rate(50_000.0) - 0.42).abs() < 1e-6);
        // Personal allowance gone at 125,140
        approx(t.personal_allowance_for(125_140.0), 0.0);
    }

    #[test]
    fn savings_tax_examples() {
        let t = TaxRules::default();
        // Basic-rate taxpayer: £1,000 PSA
        approx(t.savings_tax(30_000.0, 1_000.0), 0.0);
        approx(t.savings_tax(30_000.0, 2_000.0), 200.0);
        // Higher-rate: £500 PSA then 40%
        approx(t.savings_tax(60_000.0, 1_500.0), 400.0);
    }

    #[test]
    fn cgt_examples() {
        let t = TaxRules::default();
        // Higher-rate taxpayer: (50k - 3k) * 24%
        approx(t.cgt(50_000.0, 60_000.0), 11_280.0);
        // Lower earner uses some basic band at 18%
        let used = 30_000.0 - 12_570.0;
        let basic_left = 37_700.0 - used;
        approx(t.cgt(50_000.0, 30_000.0), basic_left * 0.18 + (47_000.0 - basic_left) * 0.24);
        assert!((t.prr_fraction(5.0, 10.0) - 0.575).abs() < 1e-9);
    }
}
