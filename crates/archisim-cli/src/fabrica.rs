//! `archisim fabrica`: what the factory needs from a project of modular
//! boxes (M16): the panels dimensioned, one A3 sheet per box; the labels of
//! every piece with their QR code, A4; the order of each box (its shopping
//! list); and the file for the roll former.

use std::path::Path;

use archisim_core::Model;

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Writes it all to `dir`; returns what it says.
pub fn write(m: &Model, dir: &Path, stem: &str) -> Res<String> {
    std::fs::create_dir_all(dir)?;
    let mut said = String::new();
    // Panels.
    let panels = archisim_plan::factory::panel_pages(m);
    if !panels.is_empty() {
        let pages: Vec<archisim_export::pdf::Page> = panels
            .iter()
            .map(|(code, svg)| archisim_export::pdf::Page { svg, title: code })
            .collect();
        let path = dir.join(format!("{stem}-paneles.pdf"));
        std::fs::write(
            &path,
            archisim_export::pdf::pdf(&pages, "Paneles de las cajas")?,
        )?;
        for (code, svg) in &panels {
            std::fs::write(dir.join(format!("{stem}-paneles-{code}.svg")), svg)?;
        }
        said.push_str(&format!(
            "Paneles acotados: {} hojas A3 en {}\n",
            panels.len(),
            path.display()
        ));
    }
    // Labels.
    let labels = archisim_plan::factory::label_pages(m);
    if !labels.is_empty() {
        let titles: Vec<String> = (1..=labels.len())
            .map(|i| format!("Etiquetas {i}"))
            .collect();
        let pages: Vec<archisim_export::pdf::Page> = labels
            .iter()
            .zip(&titles)
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        let path = dir.join(format!("{stem}-etiquetas.pdf"));
        std::fs::write(
            &path,
            archisim_export::pdf::pdf(&pages, "Etiquetas de las piezas")?,
        )?;
        said.push_str(&format!(
            "Etiquetas: {} hojas A4 en {}\n",
            labels.len(),
            path.display()
        ));
    }
    // The roll former.
    let pieces = archisim_geom::factory::all_pieces(m);
    let path = dir.join(format!("{stem}-perfiladora.csv"));
    std::fs::write(&path, archisim_geom::factory::machine_csv(m))?;
    said.push_str(&format!(
        "Perfiladora: {} piezas en {}\n",
        pieces.len(),
        path.display()
    ));
    // An order per box.
    let mut boxes: Vec<_> = m.boxes().collect();
    boxes.sort_by(|a, b| a.mark.cmp(&b.mark));
    for b in &boxes {
        let mut s = String::from("Grupo;Concepto;Unidad;Cantidad\n");
        for p in archisim_geom::boards::shopping_list(m, b.id) {
            let amount = if p.unit == "ud" {
                format!("{:.0}", p.amount)
            } else {
                format!("{:.2}", p.amount).replace('.', ",")
            };
            s.push_str(&format!("{};{};{};{}\n", p.group, p.what, p.unit, amount));
        }
        std::fs::write(dir.join(format!("{stem}-pedido-{}.csv", b.mark)), s)?;
    }
    said.push_str(&format!(
        "Pedidos: uno por caja ({}) en {}\n",
        boxes.len(),
        dir.display()
    ));
    Ok(said)
}
