//! Building services calculations on the model as drawn (instalaciones):
//!
//! - `electrical`: the electrical installation of a dwelling by the REBT
//!   (ITC-BT-10, 15, 17, 19, 25): its degree of electrification, the
//!   feeder from the meter, the main switch, the residual current devices
//!   and every circuit with its points, power, current, breaker, section,
//!   tube, length and voltage drop; and the single-line diagram that
//!   draws it (`archisim-plan::single_line`).
//!
//! It works on the board («cuadro»), the mechanisms and the appliances
//! drawn; it adds no elements. It is a design aid: the installer's
//! certificate and the project's installation are signed by their
//! authors.

pub mod electrical;
