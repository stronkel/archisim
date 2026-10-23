//! Prices of the budget (bloque 7 de la revisión del 22/10/2026): each item
//! of the measurements (a wall type, a finish, the railings…, named by its
//! key) can be given a concept of a price bank — its code, unit, summary,
//! price and description, copied into the project, so the budget does not
//! need the bank to be opened again — or a price typed by hand.
//!
//! A concept keeps how it is made up (its pieces: labour, machinery,
//! materials and percentages, with their yields and prices), as the bank
//! gives it, and the project its overheads (gastos generales), industrial
//! profit (beneficio industrial) and VAT, from which the budget goes from
//! the execution budget (PEM) to the contract one (PEC).

use super::*;

/// A piece of a concept: what one unit of it takes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PricePiece {
    pub code: String,
    pub unit: String,
    pub summary: String,
    /// FIEBDC type: 1 labour, 2 machinery, 3 material, 0 the rest.
    pub kind: u8,
    /// How much of it one unit of the concept takes (factor × yield); for
    /// a percentage, the percentage.
    pub quantity: f64,
    /// Euros per unit of the piece.
    pub price: f64,
}

impl PricePiece {
    /// A percentage of the pieces above it (indirect costs, small
    /// materials…): its code starts with `%` or its unit is `%`.
    pub fn is_percent(&self) -> bool {
        self.code.trim_start().starts_with('%') || self.unit.trim() == "%"
    }

    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            1 => "Mano de obra",
            2 => "Maquinaria",
            3 => "Material",
            _ if self.is_percent() => "Porcentaje",
            _ => "Otros",
        }
    }
}

/// The amount of each piece of a decomposition, in order: its quantity by
/// its price, or a percentage of the amounts above it.
pub fn piece_amounts(pieces: &[PricePiece]) -> Vec<f64> {
    let mut sum = 0.0;
    pieces
        .iter()
        .map(|p| {
            let a = if p.is_percent() {
                sum * p.quantity / 100.0
            } else {
                p.quantity * p.price
            };
            sum += a;
            a
        })
        .collect()
}

/// Overheads, industrial profit and VAT of the budget, as fractions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetSettings {
    /// Gastos generales (0,13: 13 %).
    pub overheads: f64,
    /// Beneficio industrial.
    pub profit: f64,
    /// IVA.
    pub vat: f64,
}

impl Default for BudgetSettings {
    fn default() -> Self {
        BudgetSettings {
            overheads: 0.13,
            profit: 0.06,
            vat: 0.21,
        }
    }
}

/// The figures of a budget from its execution budget (PEM).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetTotals {
    pub execution: f64,
    pub overheads: f64,
    pub profit: f64,
    /// PEM + GG + BI.
    pub contract: f64,
    pub vat: f64,
    /// With VAT.
    pub total: f64,
}

impl BudgetSettings {
    pub fn is_valid(&self) -> bool {
        [self.overheads, self.profit, self.vat]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }

    pub fn totals(&self, execution: f64) -> BudgetTotals {
        let overheads = execution * self.overheads;
        let profit = execution * self.profit;
        let contract = execution + overheads + profit;
        let vat = contract * self.vat;
        BudgetTotals {
            execution,
            overheads,
            profit,
            contract,
            vat,
            total: contract + vat,
        }
    }
}

/// A concept of a price bank as the project keeps it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PriceConcept {
    /// Its code in the bank ("EFF010"), or empty for a price typed by hand.
    pub code: String,
    pub unit: String,
    pub summary: String,
    /// Euros per unit.
    pub price: f64,
    /// Its long description, as the bank writes it.
    pub text: String,
    /// The bank it was taken from ("BCCA 2025"), or empty.
    pub bank: String,
    /// How it is made up; empty for a price without a decomposition.
    pub pieces: Vec<PricePiece>,
}

impl PriceConcept {
    pub fn is_valid(&self) -> bool {
        self.price.is_finite()
            && self.price >= 0.0
            && self
                .pieces
                .iter()
                .all(|p| p.quantity.is_finite() && p.price.is_finite())
    }

    /// The sum of its pieces, if it has any.
    pub fn pieces_total(&self) -> Option<f64> {
        (!self.pieces.is_empty()).then(|| piece_amounts(&self.pieces).iter().sum())
    }

    /// Its price made again from its pieces (after one is changed).
    pub fn reprice(&mut self) {
        if let Some(t) = self.pieces_total() {
            self.price = (t * 100.0).round() / 100.0;
        }
    }
}

impl Model {
    /// The concept given to an item of the measurements, by its key.
    pub fn price_of(&self, key: &str) -> Option<&PriceConcept> {
        self.prices.get(key)
    }

    /// Every item priced: (key, concept).
    pub fn prices(&self) -> impl Iterator<Item = (&String, &PriceConcept)> {
        self.prices.iter()
    }

    pub fn budget_settings(&self) -> BudgetSettings {
        self.budget
    }

    pub fn set_budget_settings(&mut self, s: BudgetSettings) -> Result<(), ModelError> {
        if !s.is_valid() {
            return Err(ModelError::InvalidShape(
                "overheads, profit and VAT go from 0 to 100 %",
            ));
        }
        self.budget = s;
        Ok(())
    }

    /// Gives an item a concept, or takes it away (`None`).
    pub fn set_price(
        &mut self,
        key: &str,
        concept: Option<PriceConcept>,
    ) -> Result<(), ModelError> {
        match concept {
            Some(c) => {
                if !c.is_valid() {
                    return Err(ModelError::NotFinite("price"));
                }
                self.prices.insert(key.to_string(), c);
            }
            None => {
                self.prices.remove(key);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_are_set_and_taken_away() {
        let mut m = Model::new("t");
        let c = PriceConcept {
            code: "EFF010".into(),
            unit: "m2".into(),
            summary: "Fábrica de ladrillo".into(),
            price: 32.5,
            ..Default::default()
        };
        m.set_price("muro:3", Some(c.clone())).unwrap();
        assert_eq!(m.price_of("muro:3"), Some(&c));
        assert!(m
            .set_price(
                "muro:3",
                Some(PriceConcept {
                    price: f64::NAN,
                    ..c
                })
            )
            .is_err());
        m.set_price("muro:3", None).unwrap();
        assert!(m.price_of("muro:3").is_none());
    }

    #[test]
    fn decomposition_and_contract_budget() {
        let piece = |code: &str, unit: &str, kind, quantity, price| PricePiece {
            code: code.into(),
            unit: unit.into(),
            summary: String::new(),
            kind,
            quantity,
            price,
        };
        let mut c = PriceConcept {
            code: "FFZ020".into(),
            unit: "m2".into(),
            price: 0.0,
            pieces: vec![
                piece("mo020", "h", 1, 0.5, 20.0),
                piece("mt04", "ud", 3, 50.0, 0.2),
                piece("%", "%", 0, 2.0, 0.0),
            ],
            ..Default::default()
        };
        // 10 + 10, and 2 % of 20.
        assert_eq!(piece_amounts(&c.pieces), vec![10.0, 10.0, 0.4]);
        c.reprice();
        assert!((c.price - 20.4).abs() < 1e-9);
        let t = BudgetSettings::default().totals(1000.0);
        assert!((t.contract - 1190.0).abs() < 1e-9);
        assert!((t.total - 1190.0 * 1.21).abs() < 1e-9);
        let mut m = Model::new("t");
        assert!(m
            .set_budget_settings(BudgetSettings {
                overheads: 2.0,
                ..Default::default()
            })
            .is_err());
    }
}
