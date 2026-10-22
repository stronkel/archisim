//! Prices of the budget (bloque 7 de la revisión del 22/10/2026): each item
//! of the measurements (a wall type, a finish, the railings…, named by its
//! key) can be given a concept of a price bank — its code, unit, summary,
//! price and description, copied into the project, so the budget does not
//! need the bank to be opened again — or a price typed by hand.

use super::*;

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
}

impl PriceConcept {
    pub fn is_valid(&self) -> bool {
        self.price.is_finite() && self.price >= 0.0
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
}
