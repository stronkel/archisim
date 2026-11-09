//! Building services calculations on the model as drawn (instalaciones):
//!
//! - `electrical`: the electrical installation of a dwelling by the REBT
//!   (ITC-BT-10, 15, 17, 19, 25): its degree of electrification, the
//!   feeder from the meter, the main switch, the residual current devices
//!   and every circuit with its points, power, current, breaker, section,
//!   tube, length and voltage drop; and the single-line diagram that
//!   draws it (`archisim-plan::single_line`).
//!
//! - `water`: the water supply by DB-HS 4: the cold and hot networks as
//!   drawn, their flows with simultaneity, the diameter of every stretch,
//!   its speed and pressure loss, and the pressure left at every tap.
//!
//! - `drain`: the drainage by DB-HS 5: waste water by units of discharge
//!   (branches, downpipes and collectors), rain water by the roof each
//!   inlet takes, mixed collectors, the outlets of every roof and its
//!   gutters.
//!
//! It works on the board («cuadro»), the mechanisms and the appliances
//! drawn; it adds no elements. It is a design aid: the installer's
//! certificate and the project's installation are signed by their
//! authors.

pub mod drain;
pub mod electrical;
pub mod envelope;
pub mod renewables;
pub mod solar;
pub mod thermal;
pub mod ventilation;
pub mod water;
