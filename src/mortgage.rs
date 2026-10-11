//! Mortgage maths: repayment and interest-only loans with an initial fixed
//! rate that reverts to a follow-on rate.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoanTerms {
    pub principal: f64,
    /// Annual rate in percent during the fixed/initial period.
    pub initial_rate_pct: f64,
    /// Length of the initial period in years.
    pub fixed_years: f64,
    /// Annual rate in percent after the initial period (e.g. lender SVR or expected remortgage rate).
    pub revert_rate_pct: f64,
    pub term_years: f64,
    pub interest_only: bool,
    /// Extra capital paid each month on top of the contractual payment.
    pub overpayment_monthly: f64,
}

impl LoanTerms {
    pub fn rate_for_month(&self, month: u32) -> f64 {
        let pct = if (month as f64) < self.fixed_years * 12.0 {
            self.initial_rate_pct
        } else {
            self.revert_rate_pct
        };
        pct / 100.0 / 12.0
    }

    /// Contractual monthly payment at the start of the loan.
    pub fn initial_payment(&self) -> f64 {
        let mut loan = Loan::new(self.clone());
        loan.current_payment()
    }

    /// Run the whole loan to the end of its term.
    pub fn full_schedule(&self) -> Vec<LoanMonth> {
        let mut loan = Loan::new(self.clone());
        let months = (self.term_years * 12.0).round() as u32;
        (0..months).map(|_| loan.step()).collect()
    }
}

/// Standard annuity payment.
pub fn annuity_payment(balance: f64, monthly_rate: f64, months_left: u32) -> f64 {
    if balance <= 0.0 || months_left == 0 {
        return balance.max(0.0);
    }
    if monthly_rate.abs() < 1e-12 {
        return balance / months_left as f64;
    }
    balance * monthly_rate / (1.0 - (1.0 + monthly_rate).powi(-(months_left as i32)))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LoanMonth {
    pub interest: f64,
    pub principal: f64,
    pub payment: f64,
    pub balance: f64,
}

/// Stateful loan that advances one month at a time.
#[derive(Clone, Debug)]
pub struct Loan {
    pub terms: LoanTerms,
    pub balance: f64,
    pub month: u32,
    pub total_interest: f64,
    /// Contractual payment, fixed until the rate changes.
    payment: Option<f64>,
}

impl Loan {
    pub fn new(terms: LoanTerms) -> Self {
        Self { balance: terms.principal.max(0.0), terms, month: 0, total_interest: 0.0, payment: None }
    }

    pub fn months_left(&self) -> u32 {
        ((self.terms.term_years * 12.0).round() as u32).saturating_sub(self.month)
    }

    /// Contractual payment due this month. It is recalculated only when the
    /// rate changes, so overpayments shorten the loan rather than lowering
    /// the payment.
    pub fn current_payment(&mut self) -> f64 {
        let r = self.terms.rate_for_month(self.month);
        if self.balance <= 0.0 {
            return 0.0;
        }
        if self.terms.interest_only {
            if self.months_left() <= 1 {
                return self.balance * (1.0 + r);
            }
            return self.balance * r;
        }
        let rate_changed = self.month > 0 && self.terms.rate_for_month(self.month - 1) != r;
        if self.payment.is_none() || rate_changed || self.months_left() <= 1 {
            self.payment = Some(annuity_payment(self.balance, r, self.months_left().max(1)));
        }
        self.payment.unwrap()
    }

    pub fn step(&mut self) -> LoanMonth {
        if self.balance <= 0.0 {
            self.month += 1;
            return LoanMonth::default();
        }
        let r = self.terms.rate_for_month(self.month);
        let interest = self.balance * r;
        let contractual = self.current_payment();
        let payment = (contractual + self.terms.overpayment_monthly).min(self.balance + interest);
        let principal = payment - interest;
        self.balance = (self.balance - principal).max(0.0);
        if self.balance < 0.005 {
            self.balance = 0.0;
        }
        self.total_interest += interest;
        self.month += 1;
        LoanMonth { interest, principal, payment, balance: self.balance }
    }

    /// Repay what is left (used when a property is sold).
    pub fn settle(&mut self) -> f64 {
        let b = self.balance;
        self.balance = 0.0;
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(rate: f64, years: f64, io: bool) -> LoanTerms {
        LoanTerms {
            principal: 200_000.0,
            initial_rate_pct: rate,
            fixed_years: years,
            revert_rate_pct: rate,
            term_years: years,
            interest_only: io,
            overpayment_monthly: 0.0,
        }
    }

    #[test]
    fn annuity_matches_known_value() {
        // £200k, 5%, 25 years ≈ £1,169.18/month
        let p = terms(5.0, 25.0, false).initial_payment();
        assert!((p - 1169.18).abs() < 0.05, "{p}");
    }

    #[test]
    fn repayment_loan_clears() {
        let s = terms(4.0, 25.0, false).full_schedule();
        assert!(s.last().unwrap().balance < 0.01);
        let interest: f64 = s.iter().map(|m| m.interest).sum();
        assert!(interest > 100_000.0 && interest < 120_000.0, "{interest}");
    }

    #[test]
    fn interest_only_keeps_balance() {
        let mut l = Loan::new(terms(6.0, 10.0, true));
        let m = l.step();
        assert!((m.interest - 1000.0).abs() < 1e-6);
        assert!((l.balance - 200_000.0).abs() < 1e-6);
    }

    #[test]
    fn overpayments_shorten_loan() {
        let mut t = terms(5.0, 25.0, false);
        t.overpayment_monthly = 300.0;
        let s = t.full_schedule();
        let paid_off = s.iter().position(|m| m.balance == 0.0).unwrap();
        assert!(paid_off < 25 * 12 - 60);
    }
}
