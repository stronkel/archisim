//! What leaves ArchiSim as drawings: plans and sheets as vector PDF (for
//! printing and sending) and as DXF (for those who go on in AutoCAD or
//! another CAD program), and DXF drawings brought in as 2D drafting.
//!
//! Both outputs start from the same SVG the screen shows
//! ([`archisim_plan`]), read into plain paths with usvg, so what is printed
//! is what is seen. The DXF also writes ArchiSim's own drafting natively
//! (lines, arcs, texts and hatches on their layers), as AutoCAD would.

pub mod dxf;
pub mod dxf_read;
pub mod paths;
pub mod pdf;
