//! The chart tabs in the central panel.

use eframe::egui::{self, Color32, RichText, Ui};
use egui_plot::{Bar, BarChart, GridMark, HLine, Heatmap, Line, LineStyle, Plot, PlotPoint, Span, Text, VLine};
use serde::{Deserialize, Serialize};

use super::*;
use house_calc::analysis::{self, BreakEvens, Grid, Param, SweepPoint};
use house_calc::model::Inputs;
use house_calc::scenario::{first_home_cgt, letting_year_one, simulate, Comparison, LettingYearOne, ScenarioResult, Strategy};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    Overview,
    Mortgages,
    Letting,
    Taxes,
    SellAndSave,
    Sensitivity,
    Considerations,
}

impl Tab {
    pub const ALL: [Tab; 7] =
        [Tab::Overview, Tab::Mortgages, Tab::Letting, Tab::Taxes, Tab::SellAndSave, Tab::Sensitivity, Tab::Considerations];
    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "📊 Overview",
            Tab::Mortgages => "🏦 Mortgages & interest",
            Tab::Letting => "🔑 Letting",
            Tab::Taxes => "⚖ Taxes",
            Tab::SellAndSave => "💰 Sell & save",
            Tab::Sensitivity => "🔍 What-if explorer",
            Tab::Considerations => "📝 Other factors",
        }
    }
}

/// Everything derived from the inputs, recomputed only when they change.
#[derive(Clone, Default)]
pub struct Computed {
    pub inputs: Option<Inputs>,
    pub cmp: Comparison,
    pub be: BreakEvens,
    pub let1: LettingYearOne,
    pub rent_curve: Vec<[f64; 2]>,
    pub savings_curves: Vec<(f64, Vec<[f64; 2]>, f64)>,
    pub savings_sweep: Vec<SweepPoint>,
    pub cgt_by_sale_year: Vec<[f64; 2]>,
}

impl Computed {
    pub fn refresh(&mut self, i: &Inputs) {
        if self.inputs.as_ref() == Some(i) {
            return;
        }
        let liquid = i.liquidate_at_end;
        self.cmp = Comparison::run(i);
        self.be = analysis::break_evens(i, liquid);
        self.let1 = letting_year_one(i, i.letting.monthly_rent);

        let r0 = i.letting.monthly_rent.max(100.0);
        self.rent_curve = (0..=60)
            .map(|k| {
                let r = r0 * (0.4 + 1.4 * k as f64 / 60.0);
                [r, letting_year_one(i, r).net_cashflow / 12.0]
            })
            .collect();

        let mut rates: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        if !rates.iter().any(|r| (r - i.savings.rate_pct).abs() < 1e-9) {
            rates.push(i.savings.rate_pct);
            rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        }
        self.savings_curves = rates
            .into_iter()
            .map(|r| {
                let mut j = i.clone();
                j.savings.rate_pct = r;
                let s = simulate(&j, Strategy::Sell);
                let pts = s.years.iter().map(|y| [y.year as f64, y.cash]).collect();
                (r, pts, s.final_net_worth(liquid))
            })
            .collect();
        self.savings_sweep = analysis::sweep(i, Param::SavingsRate, 0.0, 10.0, 41, liquid);

        let g = (1.0 + i.current.growth_pct / 100.0).ln();
        self.cgt_by_sale_year = (0..=i.horizon_years * 4)
            .map(|q| {
                let y = q as f64 / 4.0;
                let price = i.current.value * (g * y).exp();
                [y, first_home_cgt(i, price, y, &[])]
            })
            .collect();

        self.inputs = Some(i.clone());
    }
}

/// User choices inside the tabs.
#[derive(Clone, Serialize, Deserialize)]
pub struct TabState {
    pub tab: Tab,
    pub mortgage_view: Strategy,
    pub sweep_param: Param,
    pub sweep_range: (f64, f64),
    pub heat_x: Param,
    pub heat_y: Param,
    pub heat_res: usize,
    pub heat_labels: bool,
    #[serde(skip)]
    pub sweep_cache: Option<(Inputs, Param, (f64, f64), Vec<SweepPoint>)>,
    #[serde(skip)]
    pub grid_cache: Option<(Inputs, Param, Param, usize, Grid)>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            tab: Tab::Overview,
            mortgage_view: Strategy::KeepAndLet,
            sweep_param: Param::FirstHomeGrowth,
            sweep_range: (-3.0, 8.0),
            heat_x: Param::FirstHomeGrowth,
            heat_y: Param::MonthlyRent,
            heat_res: 15,
            heat_labels: false,
            sweep_cache: None,
            grid_cache: None,
        }
    }
}

fn series<'a>(name: &str, pts: Vec<[f64; 2]>, color: Color32) -> Line<'a> {
    Line::new(name.to_string(), pts).color(color).width(2.0)
}

fn by_year(r: &ScenarioResult, f: impl Fn(&house_calc::scenario::YearPoint) -> f64) -> Vec<[f64; 2]> {
    r.years.iter().map(|y| [y.year as f64, f(y)]).collect()
}

fn warnings(ui: &mut Ui, c: &Comparison) {
    for w in c.sell.warnings.iter().chain(c.keep.warnings.iter()) {
        ui.label(RichText::new(format!("⚠ {w}")).color(WARNING));
    }
}

// ------------------------------------------------------------------ Overview

pub fn overview(ui: &mut Ui, i: &Inputs, c: &Computed) {
    let liquid = i.liquidate_at_end;
    let adv = c.cmp.advantage(liquid);
    let h = i.horizon_years;

    kpi_row(ui, |t| {
        let (verdict, col) = if adv >= 0.0 { ("Keeping & letting wins", KEEP) } else { ("Selling wins", SELL) };
        kpi(t, &format!("After {h} years"), verdict, &format!("by {}", money(adv.abs())), Some(col));
        kpi(t, "Net worth – sell", &money(c.cmp.sell.final_net_worth(liquid)), "", Some(SELL));
        kpi(
            t,
            "Net worth – keep & let",
            &money(c.cmp.keep.final_net_worth(liquid)),
            if liquid { "after selling costs & CGT" } else { "before selling costs & CGT" },
            Some(KEEP),
        );
        match c.be.growth_vs_selling_pct {
            Some(g) => kpi(
                t,
                "First home must grow",
                &format!("{g:.2}% / yr"),
                &format!("to break even (you assume {:.2}%)", i.current.growth_pct),
                None,
            ),
            None => kpi(t, "First home must grow", "n/a", "no break-even in −20%…30%", None),
        }
        match c.be.rent_vs_selling {
            Some(r) => kpi(t, "Rent needed to beat selling", &format!("{}/mo", money(r)), &format!("you assume {}", money(i.letting.monthly_rent)), None),
            None => kpi(t, "Rent needed to beat selling", "n/a", "", None),
        }
        kpi(t, "Extra upfront if keeping", &money(c.cmp.keep.upfront.ads), "ADS (8% of new price)", None);
    });
    warnings(ui, &c.cmp);

    section_title(ui, "Net worth over time");
    help(ui, "Property values − mortgages + cash. Dashed: keep & let after paying selling costs and CGT on the first home.");
    money_plot("nw", "Year").show(ui, |p| {
        p.line(series("Sell", by_year(&c.cmp.sell, |y| y.net_worth), SELL));
        p.line(series("Keep & let", by_year(&c.cmp.keep, |y| y.net_worth), KEEP));
        p.line(
            series("Keep & let (if sold)", by_year(&c.cmp.keep, |y| y.net_worth_liquid), KEEP).style(LineStyle::dashed_dense()),
        );
    });

    section_title(ui, "How far ahead is keeping? (keep − sell)");
    let pts: Vec<[f64; 2]> = c
        .cmp
        .keep
        .years
        .iter()
        .zip(&c.cmp.sell.years)
        .map(|(k, s)| {
            let kv = if liquid { k.net_worth_liquid } else { k.net_worth };
            [k.year as f64, kv - s.net_worth]
        })
        .collect();
    money_plot("adv", "Year").height(220.0).show(ui, |p| {
        p.hline(HLine::new("", 0.0).color(MUTED).width(1.0));
        p.line(series("Keep − sell", pts, KEEP).fill(0.0).fill_alpha(0.15));
    });

    section_title(ui, "Completion day");
    upfront_table(ui, &c.cmp);
}

fn upfront_table(ui: &mut Ui, c: &Comparison) {
    let (s, k) = (&c.sell.upfront, &c.keep.upfront);
    egui::Grid::new("upfront").striped(true).num_columns(3).min_col_width(140.0).show(ui, |ui| {
        ui.label("");
        ui.label(RichText::new("Sell").color(SELL).strong());
        ui.label(RichText::new("Keep & let").color(KEEP).strong());
        ui.end_row();
        let rows: [(&str, f64, f64); 15] = [
            ("Cash you start with", s.starting_cash, k.starting_cash),
            ("Sale price of first home", s.sale_price, k.sale_price),
            ("Selling costs", -s.sale_costs, -k.sale_costs),
            ("Mortgage repaid", -s.mortgage_redeemed, -k.mortgage_redeemed),
            ("Equity released (BTL)", s.equity_released, k.equity_released),
            ("Remortgage fees", -s.remortgage_fees, -k.remortgage_fees),
            ("Letting setup", -s.letting_setup, -k.letting_setup),
            ("LBTT", -s.lbtt, -k.lbtt),
            ("ADS", -s.ads, -k.ads),
            ("Purchase fees", -s.purchase_fees, -k.purchase_fees),
            ("Deposit on new home", -s.deposit, -k.deposit),
            ("= Cash left", s.cash_left, k.cash_left),
            ("New mortgage", s.new_mortgage, k.new_mortgage),
            ("New mortgage LTV", s.ltv_pct, k.ltv_pct),
            ("Loan on first home", s.btl_loan, k.btl_loan),
        ];
        for (label, a, b) in rows {
            ui.label(label);
            if label.contains("LTV") {
                ui.label(format!("{a:.1}%"));
                ui.label(format!("{b:.1}%"));
            } else {
                ui.label(money(a));
                ui.label(money(b));
            }
            ui.end_row();
        }
    });
}

// ------------------------------------------------------------------ Mortgages

pub fn mortgages(ui: &mut Ui, i: &Inputs, c: &Computed, st: &mut TabState) {
    let (s, k) = (&c.cmp.sell, &c.cmp.keep);
    let first_year = |r: &ScenarioResult| r.years.get(1).cloned().unwrap_or_default();
    let s1 = first_year(s);
    let k1 = first_year(k);
    kpi_row(ui, |t| {
        kpi(t, "New mortgage – sell", &money(s.upfront.new_mortgage), &format!("{}/mo in year 1", money((s1.new_interest + s1.new_principal) / 12.0)), Some(SELL));
        kpi(t, "New mortgage – keep", &money(k.upfront.new_mortgage), &format!("{}/mo in year 1", money((k1.new_interest + k1.new_principal) / 12.0)), Some(KEEP));
        kpi(t, "Loan on first home", &money(k.upfront.btl_loan), &format!("{}/mo in year 1", money((k1.btl_interest + k1.btl_principal) / 12.0)), Some(KEEP));
        kpi(t, &format!("Interest paid over {} yrs – sell", i.horizon_years), &money(s.total_interest()), "", Some(SELL));
        kpi(t, &format!("Interest paid over {} yrs – keep", i.horizon_years), &money(k.total_interest()), "both loans", Some(KEEP));
    });

    section_title(ui, "What you owe the bank");
    money_plot("debt", "Year").include_y(0.0).legend(egui_plot::Legend::default().position(egui_plot::Corner::RightTop)).show(ui, |p| {
        p.line(series("Sell: new mortgage", by_year(s, |y| y.new_debt), SELL));
        p.line(series("Keep: new mortgage", by_year(k, |y| y.new_debt), KEEP));
        p.line(series("Keep: loan on first home", by_year(k, |y| y.first_debt), KEEP).style(LineStyle::dashed_dense()));
    });

    section_title(ui, "Cumulative interest paid");
    money_plot("cumint", "Year").height(240.0).show(ui, |p| {
        p.line(series("Sell", by_year(s, |y| y.cum_interest), SELL));
        p.line(series("Keep & let", by_year(k, |y| y.cum_interest), KEEP));
    });

    ui.horizontal(|ui| {
        section_title(ui, "Where each year's payments go");
        ui.selectable_value(&mut st.mortgage_view, Strategy::Sell, "Sell");
        ui.selectable_value(&mut st.mortgage_view, Strategy::KeepAndLet, "Keep & let");
    });
    let r = if st.mortgage_view == Strategy::Sell { s } else { k };
    let interest: Vec<Bar> = r.years.iter().skip(1).map(|y| Bar::new(y.year as f64, y.new_interest + y.btl_interest).width(0.7)).collect();
    let principal: Vec<Bar> = r.years.iter().skip(1).map(|y| Bar::new(y.year as f64, y.new_principal + y.btl_principal).width(0.7)).collect();
    let ic = BarChart::new("Interest", interest).color(SLOT5);
    let pc = BarChart::new("Capital repaid", principal).color(SLOT3).stack_on(&[&ic]);
    money_plot("split", "Year").height(240.0).show(ui, |p| {
        p.bar_chart(ic);
        p.bar_chart(pc);
    });

    section_title(ui, "Whole-term cost of the new mortgage");
    help(ui, "Total interest if you keep the new mortgage to the end of its term at the rates entered.");
    let full = |principal: f64| {
        let t = house_calc::mortgage::LoanTerms {
            principal,
            initial_rate_pct: i.new_home.rate_pct,
            fixed_years: i.new_home.fixed_years,
            revert_rate_pct: i.new_home.revert_rate_pct,
            term_years: i.new_home.term_years,
            interest_only: false,
            overpayment_monthly: i.new_home.overpayment_monthly,
        };
        let mut cum = 0.0;
        let mut pts = vec![[0.0, 0.0]];
        for (m, row) in t.full_schedule().iter().enumerate() {
            cum += row.interest;
            if m % 12 == 11 {
                pts.push([(m + 1) as f64 / 12.0, cum]);
            }
        }
        (pts, cum)
    };
    let (sp, stot) = full(s.upfront.new_mortgage);
    let (kp, ktot) = full(k.upfront.new_mortgage);
    ui.label(format!("Sell: {} interest in total · Keep: {} interest in total", money(stot), money(ktot)));
    money_plot("fullterm", "Year").height(220.0).show(ui, |p| {
        p.line(series("Sell", sp, SELL));
        p.line(series("Keep & let", kp, KEEP));
    });
}

// ------------------------------------------------------------------ Letting

pub fn letting(ui: &mut Ui, i: &Inputs, c: &Computed) {
    let l1 = &c.let1;
    let l = &i.letting;
    kpi_row(ui, |t| {
        kpi(t, "Gross yield", &format!("{:.2}%", l1.gross_yield_pct), "rent ÷ value", None);
        kpi(t, "Net yield", &format!("{:.2}%", l1.net_yield_pct), "after costs, before mortgage", None);
        let ok = l1.icr_pct >= l.lender_icr_pct;
        kpi(
            t,
            "Lender rent cover (ICR)",
            &format!("{:.0}% {}", l1.icr_pct, if ok { "✔ passes" } else { "✖ fails" }),
            &format!("needs {:.0}% at {:.2}% · max loan {}", l.lender_icr_pct, l.lender_stress_rate_pct, money(l1.max_loan_by_icr)),
            Some(if ok { GOOD } else { CRITICAL }),
        );
        let ncf = l1.net_cashflow / 12.0;
        kpi(
            t,
            "Net cash each month (yr 1)",
            &signed_money(ncf),
            "rent − costs − mortgage − tax",
            Some(if ncf >= 0.0 { GOOD } else { CRITICAL }),
        );
        if let Some(r) = c.be.rent_cashflow_neutral {
            kpi(t, "Rent to cover everything", &format!("{}/mo", money(r)), "mortgage + costs + tax", None);
        }
        if let Some(r) = c.be.rent_vs_selling {
            kpi(t, "Rent to beat selling", &format!("{}/mo", money(r)), &format!("over {} years", i.horizon_years), None);
        }
    });

    section_title(ui, "Year one: where the rent goes");
    let items = [
        ("Rent received", l1.rent, SLOT3),
        ("Running costs", -l1.costs, SLOT4),
        ("Mortgage", -l1.mortgage_payments, SLOT5),
        ("Income tax", -l1.tax, SLOT7),
        ("Net", l1.net_cashflow, if l1.net_cashflow >= 0.0 { GOOD } else { CRITICAL }),
    ];
    // Waterfall: each step floats from the running total.
    let mut running = 0.0;
    let mut charts = Vec::new();
    for (k, (name, v, col)) in items.iter().enumerate() {
        let (base, h) = if k == items.len() - 1 {
            (0.0, *v)
        } else {
            let b = running;
            running += v;
            (b, *v)
        };
        charts.push(BarChart::new(name.to_string(), vec![Bar::new(k as f64, h).base_offset(base).width(0.6).name(name)]).color(*col));
    }
    let names: Vec<String> = items.iter().map(|x| x.0.to_string()).collect();
    money_plot_bare("waterfall", "").height(260.0)
        .x_axis_formatter(move |m: GridMark, _| {
            let k = m.value.round();
            if (m.value - k).abs() < 1e-6 && k >= 0.0 && (k as usize) < names.len() { names[k as usize].clone() } else { String::new() }
        })
        .show(ui, |p| {
            p.hline(HLine::new("", 0.0).color(MUTED).width(1.0));
            for ch in charts {
                p.bar_chart(ch);
            }
        });

    section_title(ui, "Monthly cash flow at different rents (year one)");
    help(ui, "Where the line crosses zero, the rent covers the mortgage, costs and tax.");
    money_plot("rentcurve", "Rent £/month").height(260.0).show(ui, |p| {
        p.hline(HLine::new("", 0.0).color(MUTED).width(1.0));
        p.line(series("Net cash / month", c.rent_curve.clone(), KEEP));
        p.vline(VLine::new("Your rent", i.letting.monthly_rent).color(MUTED).style(LineStyle::dashed_dense()));
        if let Some(r) = c.be.rent_cashflow_neutral {
            p.vline(VLine::new("Break-even rent", r).color(GOOD));
        }
    });

    section_title(ui, "Letting year by year (keep & let)");
    let k = &c.cmp.keep;
    money_plot("letyears", "Year").height(260.0).show(ui, |p| {
        p.line(series("Rent received", by_year(k, |y| y.rent_received), SLOT3));
        p.line(series("Running costs", by_year(k, |y| y.letting_costs), SLOT4));
        p.line(series("Mortgage interest", by_year(k, |y| y.btl_interest), SLOT5));
        p.line(series("Income tax on rent", by_year(k, |y| y.rental_tax), SLOT7));
        p.line(series(
            "Profit after tax",
            k.years.iter().skip(1).map(|y| [y.year as f64, y.rent_received - y.letting_costs - y.btl_interest - y.rental_tax]).collect(),
            KEEP,
        ));
    });
}

// ------------------------------------------------------------------ Taxes

pub fn taxes(ui: &mut Ui, i: &Inputs, c: &Computed) {
    let (s, k) = (&c.cmp.sell, &c.cmp.keep);
    let t = &i.tax;
    let sum = |r: &ScenarioResult, f: fn(&house_calc::scenario::YearPoint) -> f64| r.years.iter().map(f).sum::<f64>();
    let horizon_cgt = c.cgt_by_sale_year.last().map_or(0.0, |p| p[1]);
    kpi_row(ui, |t| {
        kpi(t, "LBTT – sell", &money(s.upfront.lbtt), "", Some(SELL));
        kpi(t, "LBTT + ADS – keep", &money(k.upfront.lbtt + k.upfront.ads), &format!("ADS {}", money(k.upfront.ads)), Some(KEEP));
        kpi(t, "Income tax on rent", &money(sum(k, |y| y.rental_tax)), &format!("over {} years", i.horizon_years), Some(KEEP));
        kpi(t, "CGT if sold at the end", &money(horizon_cgt), &format!("year {}", i.horizon_years), Some(KEEP));
        kpi(t, "Total tax – sell", &money(s.total_tax()), "LBTT + savings tax", Some(SELL));
        kpi(t, "Total tax – keep", &money(k.total_tax()), "LBTT, ADS, rent, savings, CGT", Some(KEEP));
    });

    section_title(ui, "Your tax position");
    egui::Grid::new("owners").striped(true).num_columns(5).show(ui, |ui| {
        for h in ["Owner", "Income", "Scottish marginal rate", "Marginal rate with rent", "Share"] {
            ui.label(RichText::new(h).strong());
        }
        ui.end_row();
        let shares = i.shares();
        let profit = c.let1.rent - c.let1.costs;
        for (o, sh) in i.owners.iter().zip(shares) {
            ui.label(&o.name);
            ui.label(money(o.income));
            ui.label(format!("{:.0}%", t.scottish_marginal_rate(o.income) * 100.0));
            ui.label(format!("{:.0}%", t.scottish_marginal_rate(o.income + profit * sh) * 100.0));
            ui.label(format!("{:.0}%", sh * 100.0));
            ui.end_row();
        }
    });
    help(ui, "Rent profit is taxed at Scottish rates on top of your income. Mortgage interest isn't deductible – you get a 20% credit instead (Section 24), which hurts higher-rate payers.");

    section_title(ui, "Tax paid each year");
    let bars = |r: &ScenarioResult, f: fn(&house_calc::scenario::YearPoint) -> f64, off: f64| -> Vec<Bar> {
        r.years.iter().skip(1).map(|y| Bar::new(y.year as f64 + off, f(y)).width(0.38)).collect()
    };
    let k_rent = BarChart::new("Keep: tax on rent", bars(k, |y| y.rental_tax, 0.2)).color(KEEP);
    let k_cgt = BarChart::new("Keep: CGT", bars(k, |y| y.cgt, 0.2)).color(SLOT4).stack_on(&[&k_rent]);
    let k_sav = BarChart::new("Keep: tax on savings", bars(k, |y| y.savings_tax, 0.2)).color(SLOT7).stack_on(&[&k_rent, &k_cgt]);
    let s_sav = BarChart::new("Sell: tax on savings", bars(s, |y| y.savings_tax, -0.2)).color(SELL);
    money_plot("taxyears", "Year").height(260.0).show(ui, |p| {
        p.bar_chart(s_sav);
        p.bar_chart(k_rent);
        p.bar_chart(k_cgt);
        p.bar_chart(k_sav);
    });

    section_title(ui, "CGT if you sell the first home in year N");
    help(ui, "Private residence relief covers the years you lived there plus the final 9 months, so the taxable share grows the longer you let it. Shaded: ADS refund window.");
    let window = t.ads_refund_window_months as f64 / 12.0;
    money_plot("cgt", "Sold after (years)").height(260.0).show(ui, |p| {
        p.span(Span::new("ADS refund window", 0.0..=window).fill(GOOD.gamma_multiply(0.12)));
        p.line(series("CGT payable", c.cgt_by_sale_year.clone(), KEEP));
    });
}

// ------------------------------------------------------------------ Sell & save

pub fn sell_and_save(ui: &mut Ui, i: &Inputs, c: &Computed) {
    let s = &c.cmp.sell;
    kpi_row(ui, |t| {
        kpi(t, "Equity from sale", &money(s.upfront.sale_price - s.upfront.sale_costs - s.upfront.mortgage_redeemed), "after costs & mortgage", Some(SELL));
        kpi(t, "Put down as deposit", &money(s.upfront.deposit), &format!("LTV {:.0}%", s.upfront.ltv_pct), Some(SELL));
        kpi(t, "Cash left to save", &money(s.upfront.cash_left), "", Some(SELL));
        kpi(t, &format!("Cash after {} yrs", i.horizon_years), &money(s.final_point().cash), &format!("at {:.2}%", i.savings.rate_pct), Some(SELL));
        if let Some(r) = c.be.savings_rate_vs_keeping_pct {
            kpi(t, "Savings rate where selling wins", &format!("> {r:.2}%"), "vs keeping & letting", None);
        }
    });
    help(ui, "Set 'Deposit if selling' to Fixed in the sidebar to see whether a smaller deposit and more savings pays off.");

    section_title(ui, "Cash pot if you sell, at different interest rates");
    money_plot("savings", "Year").show(ui, |p| {
        let n = c.savings_curves.len();
        for (k, (r, pts, _)) in c.savings_curves.iter().enumerate() {
            let is_current = (r - i.savings.rate_pct).abs() < 1e-9;
            // One hue, light→dark for increasing rate (sequential magnitude).
            let t = if n > 1 { k as f32 / (n - 1) as f32 } else { 1.0 };
            let col = Color32::from_rgb(
                (0xb8 as f32 + (0x1d as f32 - 0xb8 as f32) * t) as u8,
                (0xd4 as f32 + (0x4f as f32 - 0xd4 as f32) * t) as u8,
                (0xf6 as f32 + (0x9a as f32 - 0xf6 as f32) * t) as u8,
            );
            let mut line = series(&format!("{r:.2}%{}", if is_current { " (yours)" } else { "" }), pts.clone(), col);
            if is_current {
                line = line.width(3.5);
            }
            p.line(line);
        }
    });

    section_title(ui, "Final advantage of keeping vs your savings rate");
    help(ui, "Above zero: keeping & letting beats selling. The higher the rate you can earn on the sale proceeds, the more selling wins.");
    let pts: Vec<[f64; 2]> = c.savings_sweep.iter().map(|p| [p.x, p.advantage]).collect();
    money_plot("savadv", "Savings rate %").height(240.0).show(ui, |p| {
        p.hline(HLine::new("", 0.0).color(MUTED).width(1.0));
        p.line(series("Keep − sell", pts, KEEP));
        p.vline(VLine::new("Your rate", i.savings.rate_pct).color(MUTED).style(LineStyle::dashed_dense()));
        if let Some(r) = c.be.savings_rate_vs_keeping_pct {
            p.vline(VLine::new("Break-even", r).color(GOOD));
        }
    });
}

// ------------------------------------------------------------------ Sensitivity

pub fn sensitivity(ui: &mut Ui, i: &Inputs, st: &mut TabState) {
    let liquid = i.liquidate_at_end;
    section_title(ui, "Sweep one variable");
    ui.horizontal(|ui| {
        let before = st.sweep_param;
        param_combo(ui, "sweep", &mut st.sweep_param);
        if before != st.sweep_param {
            st.sweep_range = st.sweep_param.default_range(i);
        }
        ui.label("from");
        ui.add(egui::DragValue::new(&mut st.sweep_range.0).speed(0.1));
        ui.label("to");
        ui.add(egui::DragValue::new(&mut st.sweep_range.1).speed(0.1));
        if ui.button("Auto range").clicked() {
            st.sweep_range = st.sweep_param.default_range(i);
        }
    });
    let key_ok = matches!(&st.sweep_cache, Some((ci, p, r, _)) if ci == i && *p == st.sweep_param && *r == st.sweep_range);
    if !key_ok {
        let pts = analysis::sweep(i, st.sweep_param, st.sweep_range.0, st.sweep_range.1, 61, liquid);
        st.sweep_cache = Some((i.clone(), st.sweep_param, st.sweep_range, pts));
    }
    let pts = &st.sweep_cache.as_ref().unwrap().3;
    let current = st.sweep_param.get(i);
    let crossing = pts.windows(2).find(|w| w[0].advantage.signum() != w[1].advantage.signum()).map(|w| {
        let (a, b) = (&w[0], &w[1]);
        a.x + (b.x - a.x) * (-a.advantage) / (b.advantage - a.advantage)
    });
    if let Some(x) = crossing {
        ui.label(RichText::new(format!("Break-even at {} = {:.2}", st.sweep_param.label(), x)).strong());
    } else {
        ui.label("No break-even in this range – one strategy wins throughout.");
    }
    let xl = st.sweep_param.label();
    ui.columns(2, |cols| {
        section_title(&mut cols[0], &format!("Net worth after {} years", i.horizon_years));
        money_plot("sweepnw", xl).height(280.0).show(&mut cols[0], |p| {
            p.line(series("Sell", pts.iter().map(|q| [q.x, q.sell]).collect(), SELL));
            p.line(series("Keep & let", pts.iter().map(|q| [q.x, q.keep]).collect(), KEEP));
            p.vline(VLine::new("Current", current).color(MUTED).style(LineStyle::dashed_dense()));
        });
        section_title(&mut cols[1], "Keep − sell");
        money_plot("sweepadv", xl).height(280.0).show(&mut cols[1], |p| {
            p.hline(HLine::new("", 0.0).color(MUTED).width(1.0));
            p.line(series("Keep − sell", pts.iter().map(|q| [q.x, q.advantage]).collect(), KEEP).fill(0.0).fill_alpha(0.15));
            p.vline(VLine::new("Current", current).color(MUTED).style(LineStyle::dashed_dense()));
            if let Some(x) = crossing {
                p.vline(VLine::new("Break-even", x).color(GOOD));
            }
        });
    });

    ui.separator();
    section_title(ui, "Map two variables at once");
    help(ui, "Each tile is keep − sell net worth at the horizon. Orange = keeping & letting wins, blue = selling wins, grey ≈ even.");
    ui.horizontal(|ui| {
        ui.label("Across:");
        param_combo(ui, "hx", &mut st.heat_x);
        ui.label("Up:");
        param_combo(ui, "hy", &mut st.heat_y);
        ui.add(egui::Slider::new(&mut st.heat_res, 5..=30).text("resolution"));
        ui.checkbox(&mut st.heat_labels, "labels");
    });
    let key_ok = matches!(&st.grid_cache, Some((ci, x, y, r, _)) if ci == i && *x == st.heat_x && *y == st.heat_y && *r == st.heat_res);
    if !key_ok {
        let g = analysis::grid(i, st.heat_x, st.heat_x.default_range(i), st.heat_y, st.heat_y.default_range(i), st.heat_res, st.heat_res, liquid);
        st.grid_cache = Some((i.clone(), st.heat_x, st.heat_y, st.heat_res, g));
    }
    let g = &st.grid_cache.as_ref().unwrap().4;
    heatmap(ui, i, g, st.heat_labels);
}

fn heatmap(ui: &mut Ui, i: &Inputs, g: &Grid, labels: bool) {
    let (Some(px), Some(py)) = (g.px, g.py) else { return };
    let dx = (g.x_range.1 - g.x_range.0) / (g.nx - 1) as f64;
    let dy = (g.y_range.1 - g.y_range.0) / (g.ny - 1) as f64;
    // Diverging scale: blue (sell wins) – grey (even) – orange (keep wins).
    let max_abs = g.values.iter().fold(1.0f64, |m, v| m.max(v.abs()));
    let mapping = move |v: f64| -> Color32 {
        let t = (v / max_abs).clamp(-1.0, 1.0) as f32;
        let (pole, a) = if t >= 0.0 { (KEEP, t) } else { (SELL, -t) };
        lerp(NEUTRAL_MID, pole, a.powf(0.7))
    };
    // Tiles are laid out in index space; axes translate back to real values.
    let hm = Heatmap::new(g.values.clone(), g.nx)
        .at(PlotPoint::new(-0.5, -0.5))
        .tile_size(1.0, 1.0)
        .show_labels(labels)
        .formatter(Box::new(money_short))
        .custom_mapping(Box::new(mapping));
    let (x0, y0) = (g.x_range.0, g.y_range.0);
    let fx = move |k: f64| x0 + k * dx;
    let fy = move |k: f64| y0 + k * dy;
    let (cx, cy) = ((px.get(i) - x0) / dx, (py.get(i) - y0) / dy);
    let (nx, ny) = (g.nx, g.ny);
    let values = g.values.clone();
    Plot::new("heat")
        .height(460.0)
        .allow_scroll(false)
        .allow_zoom(false)
        .allow_drag(false)
        .show_grid(false)
        .x_axis_label(px.label())
        .y_axis_label(py.label())
        .x_axis_formatter(move |m: GridMark, _| fmt_param(px, fx(m.value)))
        .y_axis_formatter(move |m: GridMark, _| fmt_param(py, fy(m.value)))
        .label_formatter(move |hp: &egui_plot::HoverPosition<'_>| {
            let p = match hp {
                egui_plot::HoverPosition::NearDataPoint { position, .. } | egui_plot::HoverPosition::Elsewhere { position } => *position,
            };
            let (kx, ky) = (p.x.round(), p.y.round());
            if kx < 0.0 || ky < 0.0 || kx as usize >= nx || ky as usize >= ny {
                return None;
            }
            let v = values[ky as usize * nx + kx as usize];
            let who = if v >= 0.0 { "keeping wins by" } else { "selling wins by" };
            Some(format!(
                "{}: {}\n{}: {}\n{who} {}",
                px.label(),
                fmt_param(px, fx(kx)),
                py.label(),
                fmt_param(py, fy(ky)),
                money(v.abs())
            ))
        })
        .show(ui, |p| {
            p.heatmap(hm);
            p.points(
                egui_plot::Points::new("You are here", vec![[cx, cy]])
                    .shape(egui_plot::MarkerShape::Cross)
                    .radius(9.0)
                    .color(Color32::WHITE),
            );
            p.text(Text::new("", PlotPoint::new(cx, cy + 0.45), RichText::new("you").strong().color(Color32::WHITE)));
        });
}

fn fmt_param(p: Param, v: f64) -> String {
    match p {
        Param::MonthlyRent | Param::ReleaseEquity | Param::NewHomePrice => money_short(v),
        Param::SaleYear => format!("{v:.1} yrs"),
        _ => format!("{v:.1}%"),
    }
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

fn param_combo(ui: &mut Ui, id: &str, p: &mut Param) {
    egui::ComboBox::from_id_salt(id).selected_text(p.label()).width(240.0).show_ui(ui, |ui| {
        for q in Param::ALL {
            ui.selectable_value(p, q, q.label());
        }
    });
}

// ------------------------------------------------------------------ Considerations

pub fn considerations(ui: &mut Ui, i: &Inputs, c: &Computed) {
    let k = &c.cmp.keep;
    section_title(ui, "Things the numbers don't fully capture");
    let mut items: Vec<(String, String)> = vec![
        (
            "ADS refund deadline".into(),
            format!(
                "If you buy first and sell the old home within 36 months you can reclaim the {} ADS from Revenue Scotland. Diary the deadline and claim within 5 years of the new purchase.",
                money(k.upfront.ads)
            ),
        ),
        ("Landlord registration (Scotland)".into(), "You must register with your council before letting. Use an approved tenancy deposit scheme, and provide gas safety, EICR, smoke/heat/CO alarms and Legionella risk assessment.".into()),
        ("Private Residential Tenancy".into(), "Scottish tenancies are open-ended. You can only end one on specific grounds (e.g. you intend to sell), with notice – plan ahead if you might want to sell.".into()),
        ("Rent controls".into(), "The Housing (Scotland) Act 2025 allows rent control areas where increases can be capped. Check whether your area is affected before relying on rent growth.".into()),
        ("EPC requirements".into(), "Minimum energy standards for private lets are being tightened – budget for upgrades if the first home has a low EPC rating.".into()),
        ("Mortgage consent".into(), "Letting without a buy-to-let mortgage needs your lender's consent-to-let, often with a rate loading.".into()),
        ("CGT reporting".into(), "If you sell a let property with a gain you must report and pay CGT within 60 days of completion.".into()),
        ("Property income tax changes".into(), "The UK government has announced separate (higher) property income tax rates from April 2027 for England, Wales & NI; the Scottish Government sets its own – update the Tax rules section if Scotland follows.".into()),
        ("Voids, arrears and repairs".into(), "A bad tenant or a large repair (roof, boiler) can wipe out several years of profit. Keep a landlord reserve fund.".into()),
        ("Concentration risk".into(), "Keeping two homes means more of your wealth is in property and more debt – a price fall hurts twice as much.".into()),
        ("Your time".into(), "Self-managing saves the agent fee but means calls at night, viewings and paperwork.".into()),
        ("Self Assessment".into(), "Rental income means filing a tax return each year; tax may be due in payments on account.".into()),
    ];
    if i.new_home.first_time_buyer {
        items.insert(0, ("First-time buyer relief".into(), "You own a home now, so you can't claim first-time buyer relief on the new one – untick it.".into()));
    }
    if c.let1.icr_pct < i.letting.lender_icr_pct && i.letting.remortgage_to_btl {
        items.insert(0, ("Lender rent cover".into(), format!("At {} rent the buy-to-let lender's stress test fails; maximum loan would be about {}.", money(i.letting.monthly_rent), money(c.let1.max_loan_by_icr))));
    }
    for (title, body) in items {
        egui::Frame::group(ui.style()).inner_margin(8.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).strong());
            ui.label(body);
        });
    }
    ui.add_space(8.0);
    help(ui, "This tool is a model to help you think, not financial or tax advice. Check figures with a mortgage adviser and a tax adviser before you commit.");
}
